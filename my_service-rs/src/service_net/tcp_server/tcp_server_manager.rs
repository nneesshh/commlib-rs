use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;
use thread_local::ThreadLocal;

use commlib::with_tls_mut;
use net_packet::NetPacketGuard;
use pinky_swear::PinkySwear;

use crate::service_net::packet_builder::tcp_packet_builder::PacketBuilder;
use crate::service_net::service_net_impl::create_tcp_server;
use crate::service_net::tcp_conn_manager::{insert_connection, on_connection_established};
use crate::{ConnId, PacketType, ServiceNetRs, ServiceRs, TcpConn};

use super::tcp_server_impl::TcpServer;

thread_local! {
    static G_TCP_SERVER_STORAGE: UnsafeCell<TcpServerStorage> = UnsafeCell::new(TcpServerStorage::new());
}

struct TcpServerStorage {
    /// tcp server vector
    tcp_server_vec: Vec<Arc<TcpServer>>,
}

impl TcpServerStorage {
    ///
    pub fn new() -> Self {
        Self {
            tcp_server_vec: Vec::new(),
        }
    }
}

/// Listen on [ip:port] over service net
pub fn tcp_server_listen<T, C, P, S>(
    srv: &Arc<T>,
    addr: &str,
    conn_fn: C,
    pkt_fn: P,
    close_fn: S,
    connection_limit: usize,
    srv_net: &Arc<ServiceNetRs>,
) -> bool
where
    T: ServiceRs + 'static,
    C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
    P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    log::info!("tcp_server_listen: {}...", addr);

    let (promise, pinky) = PinkySwear::<bool>::new();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let srv2 = srv.clone();
    let addr2 = addr.to_owned();

    let func = move || {
        //
        let tcp_server = Arc::new(create_tcp_server(
            &srv2,
            addr2.as_str(),
            conn_fn,
            pkt_fn,
            close_fn,
            connection_limit,
            &srv_net2,
        ));

        // listen
        tcp_server.listen(&srv2, |listener_id, sock_addr| {
            //
            log::info!(
                "tcp_server_listen: listener_id={} sock_addr={} ready.",
                listener_id,
                sock_addr
            );
        });

        // add tcp server to serivce net
        with_tls_mut!(G_TCP_SERVER_STORAGE, g, {
            g.tcp_server_vec.push(tcp_server);
        });

        //
        pinky.swear(true);
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait()
}

/// Notify tcp server to stop
pub fn notify_tcp_server_stop(srv_net: &ServiceNetRs) {
    log::info!("notify_tcp_server_stop ...");

    // 投递到 srv_net 线程
    let (promise, pinky) = PinkySwear::<bool>::new();
    let func = move || {
        with_tls_mut!(G_TCP_SERVER_STORAGE, g, {
            for tcp_server in &mut g.tcp_server_vec {
                tcp_server.stop();
            }
        });

        pinky.swear(true);
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait();
}

/// Make new tcp conn with callbacks from tcp server
pub fn tcp_server_make_new_conn(tcp_server: &Arc<TcpServer>, hd: ConnId, sock_addr: SocketAddr) {
    // 运行于 srv_net 线程
    assert!(tcp_server.srv_net().is_in_service_thread());

    let packet_type = PacketType::Server;
    let packet_builder: Arc<ThreadLocal<UnsafeCell<PacketBuilder>>> = Arc::new(ThreadLocal::new());

    // 连接数量
    if tcp_server.check_connection_limit() {
        log::error!("connection overflow!!! tcp_server_make_new_conn failed!!!");
        return;
    }

    // event handler: connect
    let tcp_server2 = tcp_server.clone();
    let connection_establish_fn = Box::new(move |conn: Arc<TcpConn>| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());

        tcp_server2.on_ll_connect(conn);
    });

    // event handler: input( use packet builder to handle input buffer )
    let tcp_server2 = tcp_server.clone();
    let connection_read_fn = Box::new(move |conn: Arc<TcpConn>, input_buffer: NetPacketGuard| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());

        let builder = get_packet_builder(&packet_builder, &tcp_server2);
        tcp_server2.on_ll_input(conn, input_buffer, builder);
    });

    // event handler: disconnect
    let tcp_server2 = tcp_server.clone();
    let connection_lost_fn = Arc::new(move |hd: ConnId| {
        // 运行于 srv_net 线程
        assert!(tcp_server2.srv_net().is_in_service_thread());

        tcp_server2.on_ll_disconnect(hd);
    });

    //
    let netctrl = tcp_server.netctrl().clone();
    let srv_net = tcp_server.srv_net().clone();

    let conn = Arc::new(TcpConn {
        //
        hd,

        //
        sock_addr,

        //
        packet_type: Atomic::new(packet_type),
        closed: Atomic::new(false),

        //
        netctrl_opt: Some(netctrl.clone()),
        srv_net_opt: Some(srv_net.clone()),

        //
        connection_establish_fn,
        connection_read_fn,
        connection_lost_fn: RwLock::new(connection_lost_fn),
    });

    // add conn
    insert_connection(&srv_net, hd, &conn);

    // connection ok
    on_connection_established(conn);
}

#[inline(always)]
fn get_packet_builder<'a>(
    tls_pkt_builder: &'a Arc<ThreadLocal<UnsafeCell<PacketBuilder>>>,
    tcp_server: &Arc<TcpServer>,
) -> &'a mut PacketBuilder {
    // 运行于 srv_net 线程
    assert!(tcp_server.srv_net().is_in_service_thread());

    let builder = tls_pkt_builder.get_or(|| {
        let srv = tcp_server.srv().clone();
        let pkt_fn = tcp_server.pkt_fn_clone();

        let build_cb = move |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
            // 运行于 srv_net 线程
            assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());

            // post 到指定 srv 工作线程中执行
            let pkt_fn2 = pkt_fn.clone();
            srv.run_in_service(Box::new(move || {
                (*pkt_fn2)(conn, pkt);
            }));
        };
        UnsafeCell::new(PacketBuilder::new(Box::new(build_cb)))
    });
    unsafe { &mut *(builder.get()) }
}
