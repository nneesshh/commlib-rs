use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::{PinkySwear, ServiceNetRs, ServiceRs};
use crate::{TcpConn, TcpListenerId};

thread_local! {
    static G_HTTP_SERVER_STORAGE: UnsafeCell<HttpServerStorage> = UnsafeCell::new(HttpServerStorage::new());
}

struct HttpServerStorage {
    /// http server vector
    http_server_vec: Vec<Arc<HttpServer>>,
}

impl HttpServerStorage {
    ///
    pub fn new() -> Self {
        Self {
            http_server_vec: Vec::new(),
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
        let mut http_server =
            create_http_server(&srv2, addr.as_str(), conn_fn, pkt_fn, close_fn, &srv_net2);

        // listen
        http_server.listen();

        //
        let listener_id = http_server.listener_id;

        // add http server to serivce net
        with_tls_mut!(G_TCP_SERVER_STORAGE, g, {
            g.http_server_vec.push(Arc::new(http_server));
        });

        // pinky for listener_id
        pinky.swear(listener_id);
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait()
}

/// Notify http server to stop
pub fn notify_http_server_stop(srv_net: &ServiceNetRs) {
    log::info!("notify_http_server_stop ...");

    // 投递到 srv_net 线程
    let (promise, pinky) = PinkySwear::<bool>::new();
    let func = move || {
        with_tls_mut!(G_TCP_SERVER_STORAGE, g, {
            for http_server in &mut g.http_server_vec {
                http_server.stop();
            }
        });

        pinky.swear(true);
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait();
}

/// Make new tcp conn with callbacks from http server
pub fn http_server_make_new_conn(
    srv_net0: &Arc<ServiceNetRs>,
    listener_id: TcpListenerId,
    packet_type: PacketType,
    hd: ConnId,
    sock_addr: SocketAddr,
) {
    // 运行于 srv_net 线程
    assert!(srv_net0.is_in_service_thread());

    //
    with_tls_mut!(G_HTTP_SERVER_STORAGE, g, {
        // check listener id
        let mut http_server_opt: Option<&Arc<HttpServer>> = None;
        for http_server in &g.http_server_vec {
            if http_server.listener_id == listener_id {
                http_server_opt = Some(http_server);
                break;
            }
        }

        // 根据 http server 创建 tcp conn
        if let Some(http_server) = http_server_opt {
            //
            let http_server2 = http_server.clone();
            let connection_establish_fn = Box::new(move |conn: Arc<TcpConn>| {
                // 运行于 srv_net 线程
                assert!(conn.srv_net.is_in_service_thread());
                http_server2.on_ll_connect(conn);
            });

            // use packet builder to handle input buffer
            let http_server2 = http_server.clone();
            let connection_read_fn =
                Box::new(move |conn: Arc<TcpConn>, input_buffer: NetPacketGuard| {
                    // 运行于 srv_net 线程
                    assert!(conn.srv_net.is_in_service_thread());
                    http_server2.on_ll_input(conn, input_buffer);
                });

            //
            let http_server2 = http_server.clone();
            let connection_lost_fn = Arc::new(move |hd: ConnId| {
                // 运行于 srv_net 线程
                assert!(http_server2.srv_net().is_in_service_thread());
                http_server2.on_ll_disconnect(hd);
            });

            //
            let netctrl = http_server.netctrl().clone();
            let srv_net = http_server.srv_net().clone();
            let srv = http_server.srv().clone();

            let conn = Arc::new(TcpConn {
                //
                hd,

                //
                sock_addr,
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
}
