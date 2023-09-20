use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::sync::Arc;

use message_io::network::Endpoint;
use message_io::node::NodeHandler;

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
    tcp_server_vec: Vec<TcpServer>,
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

    // 在 srv_net 中运行
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
            g.tcp_server_vec.push(tcp_server);
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

    // 在 srv_net 中运行
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
    srv_net: &Arc<ServiceNetRs>,
    listener_id: TcpListenerId,
    packet_type: PacketType,
    hd: ConnId,
    endpoint: Endpoint,
    netctrl: &NodeHandler<()>,
) {
    // 在 srv_net 中运行 (同一线程便于观察 conn 生命周期)
    let srv_net2 = srv_net.clone();
    let netctrl2 = netctrl.clone();

    let func = move || {
        let mut conn_opt: Option<Arc<TcpConn>> = None;

        with_tls_mut!(G_TCP_SERVER_STORAGE, g, {
            // check listener id
            let mut tcp_server_opt: Option<&TcpServer> = None;
            for tcp_server in &g.tcp_server_vec {
                if tcp_server.listener_id == listener_id {
                    assert!(std::ptr::eq(&*tcp_server.srv_net, &*srv_net2));
                    tcp_server_opt = Some(tcp_server);
                }
            }

            // 根据 tcp server 创建 tcp conn
            if let Some(tcp_server) = tcp_server_opt {
                //
                let srv = tcp_server.srv.clone();

                //
                let conn_fn = tcp_server.clone_conn_fn();
                let connection_establish_fn = Box::new(move |conn| {
                    (*conn_fn)(conn);
                });

                // use packet builder to handle input buffer
                let srv_net3 = srv_net2.clone();
                let pkt_fn = tcp_server.clone_pkt_fn();
                let pkt_builder = PacketBuilder::new(pkt_fn);
                let connection_read_fn =
                    Box::new(move |conn: &Arc<TcpConn>, input_buffer: NetPacketGuard| {
                        pkt_builder.build(srv_net3.as_ref(), conn, input_buffer)
                    });

                //
                let close_fn = tcp_server.clone_close_fn();
                let connection_lost_fn = Arc::new(move |hd| {
                    // close fn
                    (*close_fn)(hd);
                });

                let conn = Arc::new(TcpConn {
                    //
                    hd,

                    //
                    endpoint,
                    netctrl: netctrl2.clone(),

                    //
                    packet_type: Atomic::new(packet_type),
                    closed: Atomic::new(false),

                    //
                    srv: srv.clone(),
                    srv_net: srv_net2.clone(),

                    //
                    connection_establish_fn,
                    connection_read_fn,
                    connection_lost_fn: RwLock::new(connection_lost_fn),
                });

                //
                conn_opt = Some(conn);
            }
        });

        if let Some(conn) = conn_opt {
            // add conn
            insert_connection(&srv_net2, hd, &conn);

            // connection ok
            on_connection_established(srv_net2.as_ref(), conn);
        }
    };
    srv_net.run_in_service(Box::new(func));
}
