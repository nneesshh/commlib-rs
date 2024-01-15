use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;
use thread_local::ThreadLocal;

use commlib::with_tls_mut;
use net_packet::NetPacketGuard;
use pinky_swear::PinkySwear;

use crate::service_net::packet_builder::ws_packet_builder::WsPacketBuilder;
use crate::service_net::service_net_impl::create_websocket_server;
use crate::service_net::tcp_conn_manager::{insert_connection, on_connection_established};
use crate::{ConnId, PacketType, ServiceNetRs, ServiceRs, TcpConn};

use super::ws_server_impl::WsServer;

const CONNECTION_LIMIT: usize = 10000;

thread_local! {
    static G_WS_SERVER_STORAGE: UnsafeCell<WsServerStorage> = UnsafeCell::new(WsServerStorage::new());
}

struct WsServerStorage {
    /// websocket server vector
    ws_server_vec: Vec<Arc<WsServer>>,
}

impl WsServerStorage {
    ///
    pub fn new() -> Self {
        Self {
            ws_server_vec: Vec::new(),
        }
    }
}

/// Listen on [ip:port] over service net
pub fn ws_server_listen<T, C, P, S>(
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
    log::info!("ws_server_listen: {}...", addr);

    let (promise, pinky) = PinkySwear::<bool>::new();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let srv2 = srv.clone();
    let addr2 = addr.to_owned();

    let func = move || {
        //
        let ws_server = Arc::new(create_websocket_server(
            &srv2,
            addr2.as_str(),
            conn_fn,
            pkt_fn,
            close_fn,
            connection_limit,
            &srv_net2,
        ));

        // listen
        ws_server.listen(&srv2, |listener_id, sock_addr| {
            //
            log::info!(
                "ws_server_listen: listener_id={} sock_addr={} ready.",
                listener_id,
                sock_addr
            );
        });

        // add tcp server to serivce net
        with_tls_mut!(G_WS_SERVER_STORAGE, g, {
            g.ws_server_vec.push(ws_server);
        });

        //
        pinky.swear(true);
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait()
}

/// Notify http server to stop
pub fn notify_ws_server_stop(srv_net: &ServiceNetRs) {
    log::info!("notify_ws_server_stop ...");

    // 投递到 srv_net 线程
    let (promise, pinky) = PinkySwear::<bool>::new();
    let func = move || {
        with_tls_mut!(G_WS_SERVER_STORAGE, g, {
            for ws_server in &mut g.ws_server_vec {
                ws_server.stop();
            }
        });

        pinky.swear(true);
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait();
}

/// Make new http conn with callbacks from http server
//#[inline(always)]
pub fn ws_server_make_new_conn(ws_server: &Arc<WsServer>, hd: ConnId, sock_addr: SocketAddr) {
    // 运行于 srv_net 线程
    assert!(ws_server.srv_net().is_in_service_thread());

    // 运行于 srv_net 线程
    assert!(ws_server.srv_net().is_in_service_thread());

    let packet_type = PacketType::Server;
    let packet_builder: Arc<ThreadLocal<UnsafeCell<WsPacketBuilder>>> =
        Arc::new(ThreadLocal::new());

    // 连接数量
    if ws_server.check_connection_limit() {
        log::error!("connection overflow!!! ws_server_make_new_conn failed!!!");
        return;
    }

    // event handler: connect
    let ws_server2 = ws_server.clone();
    let connection_establish_fn = Box::new(move |conn: Arc<TcpConn>| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net.is_in_service_thread());

        ws_server2.on_ll_connect(conn);
    });

    // event handler: input( use packet builder to handle input buffer )
    let ws_server2 = ws_server.clone();
    let connection_read_fn = Box::new(move |conn: Arc<TcpConn>, input_buffer: NetPacketGuard| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net.is_in_service_thread());

        let builder = get_packet_builder(&packet_builder, &ws_server2);
        ws_server2.on_ll_input(conn, input_buffer, builder);
    });

    // event handler: disconnect
    let ws_server2 = ws_server.clone();
    let connection_lost_fn = Arc::new(move |hd: ConnId| {
        // 运行于 srv_net 线程
        assert!(ws_server2.srv_net().is_in_service_thread());

        ws_server2.on_ll_disconnect(hd);
    });

    //
    let netctrl = ws_server.netctrl().clone();
    let srv_net = ws_server.srv_net().clone();

    let conn = Arc::new(TcpConn {
        //
        hd,

        //
        sock_addr,

        //
        packet_type: Atomic::new(packet_type),
        closed: Atomic::new(false),

        //
        netctrl: netctrl.clone(),
        srv_net: srv_net.clone(),

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
    tls_pkt_builder: &'a Arc<ThreadLocal<UnsafeCell<WsPacketBuilder>>>,
    tcp_server: &Arc<WsServer>,
) -> &'a mut WsPacketBuilder {
    // 运行于 srv_net 线程
    assert!(tcp_server.srv_net().is_in_service_thread());

    let builder = tls_pkt_builder.get_or(|| {
        let srv = tcp_server.srv().clone();
        let pkt_fn = tcp_server.pkt_fn_clone();

        let build_cb = move |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
            // 运行于 srv_net 线程
            assert!(conn.srv_net.is_in_service_thread());

            // post 到指定 srv 工作线程中执行
            let pkt_fn2 = pkt_fn.clone();
            srv.run_in_service(Box::new(move || {
                (*pkt_fn2)(conn, pkt);
            }));
        };
        UnsafeCell::new(WsPacketBuilder::new(Box::new(build_cb)))
    });
    unsafe { &mut *(builder.get()) }
}
