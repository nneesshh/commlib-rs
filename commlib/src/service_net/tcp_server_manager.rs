use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::sync::Arc;

use message_io::network::Endpoint;

use crate::{PinkySwear, ServiceNetRs, ServiceRs, TcpListenerId};

use super::create_tcp_server;
use super::tcp_conn_manager::{insert_connection, on_connection_established};
use super::{ConnId, TcpConn, TcpServer};
use super::{NetPacketGuard, PacketBuilder, PacketType};

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
pub fn listen_tcp_addr<T, C, P, S>(
    srv: &Arc<T>,
    ip: String,
    port: u16,
    conn_fn: C,
    pkt_fn: P,
    close_fn: S,
    srv_net: &Arc<ServiceNetRs>,
) -> TcpListenerId
where
    T: ServiceRs + 'static,
    C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
    P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    log::info!("listen_tcp_addr: {}:{}...", ip, port);

    let (promise, pinky) = PinkySwear::<TcpListenerId>::new();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let srv2 = srv.clone();
    let func = move || {
        //
        let addr = std::format!("{}:{}", ip, port);
        let mut tcp_server =
            create_tcp_server(&srv2, addr.as_str(), conn_fn, pkt_fn, close_fn, &srv_net2);

        // listen
        tcp_server.listen();

        //
        let listener_id = tcp_server.listener_id;

        // add tcp server to serivce net
        with_tls_mut!(G_TCP_SERVER_STORAGE, g, {
            g.tcp_server_vec.push(Arc::new(tcp_server));
        });

        // pinky for listener_id
        pinky.swear(listener_id);
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
pub fn tcp_server_make_new_conn(
    srv_net0: &Arc<ServiceNetRs>,
    listener_id: TcpListenerId,
    packet_type: PacketType,
    hd: ConnId,
    endpoint: Endpoint,
) {
    // 投递到 srv_net 线程 (同一线程便于观察 conn 生命周期)
    let func = move || {
        //
        with_tls_mut!(G_TCP_SERVER_STORAGE, g, {
            // check listener id
            let mut tcp_server_opt: Option<&Arc<TcpServer>> = None;
            for tcp_server in &g.tcp_server_vec {
                if tcp_server.listener_id == listener_id {
                    tcp_server_opt = Some(tcp_server);
                    break;
                }
            }

            // 根据 tcp server 创建 tcp conn
            if let Some(tcp_server) = tcp_server_opt {
                //
                let tcp_server2 = tcp_server.clone();
                let connection_establish_fn = Box::new(move |conn: Arc<TcpConn>| {
                    // 运行于 srv_net 线程
                    assert!(conn.srv_net.is_in_service_thread());
                    tcp_server2.on_ll_connect(conn);
                });

                // use packet builder to handle input buffer
                let tcp_server2 = tcp_server.clone();
                let pkt_fn = Arc::new(move |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
                    // 运行于 srv_net 线程
                    assert!(conn.srv_net.is_in_service_thread());
                    tcp_server2.on_ll_receive_packet(conn, pkt);
                });
                let pkt_builder = PacketBuilder::new(pkt_fn);
                let connection_read_fn =
                    Box::new(move |conn: &Arc<TcpConn>, input_buffer: NetPacketGuard| {
                        // 运行于 srv_net 线程
                        assert!(conn.srv_net.is_in_service_thread());
                        pkt_builder.build(conn, input_buffer)
                    });

                //
                let tcp_server2 = tcp_server.clone();
                let connection_lost_fn = Arc::new(move |hd: ConnId| {
                    // 运行于 srv_net 线程
                    assert!(tcp_server2.srv_net().is_in_service_thread());
                    tcp_server2.on_ll_disconnect(hd);
                });

                //
                let netctrl = tcp_server.netctrl().clone();
                let srv_net = tcp_server.srv_net().clone();
                let srv = tcp_server.srv().clone();

                let conn = Arc::new(TcpConn {
                    //
                    hd,

                    //
                    endpoint,
                    netctrl: netctrl.clone(),

                    //
                    packet_type: Atomic::new(packet_type),
                    closed: Atomic::new(false),

                    //
                    srv: srv.clone(),
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
        });
    };
    srv_net0.run_in_service(Box::new(func));
}
