use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;
use thread_local::ThreadLocal;

use commlib::with_tls_mut;
use net_packet::NetPacketGuard;
use pinky_swear::PinkySwear;

use crate::service_net::service_net_impl::create_http_server;
use crate::service_net::tcp_conn_manager::{insert_connection, on_connection_established};
use crate::{ConnId, PacketType, ServiceNetRs, ServiceRs, TcpConn};

use super::http_server_impl::{HttpServer, ResponseResult};
use super::request_parser::RequestParser;

const CONNECTION_LIMIT: usize = 10000;

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
pub fn http_server_listen<F>(addr: &str, request_fn: F, srv_net: &Arc<ServiceNetRs>) -> bool
where
    F: Fn(Arc<TcpConn>, http::Request<Vec<u8>>, http::response::Builder) -> ResponseResult
        + Send
        + Sync
        + 'static,
{
    log::info!("http_server_listen: {} ...", addr);

    let conn_fn = |_conn: Arc<TcpConn>| {
        //let hd = _conn.hd;
        //log::info!("[hd={}] conn_fn", hd);
    };

    let close_fn = |_hd: ConnId| {
        //log::info!("[hd={}] close_fn", _hd);
    };

    let (promise, pinky) = PinkySwear::<bool>::new();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let addr2 = addr.to_owned();

    let func = move || {
        //
        let http_server = Arc::new(create_http_server(
            addr2.as_str(),
            conn_fn,
            request_fn,
            close_fn,
            CONNECTION_LIMIT,
            &srv_net2,
        ));

        // listen
        http_server.listen();

        // add tcp server to serivce net
        with_tls_mut!(G_HTTP_SERVER_STORAGE, g, {
            g.http_server_vec.push(http_server);
        });

        //
        pinky.swear(true);
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait()
}

/// Listen with ssl on [ip:port] over service net
#[cfg(feature = "ssl")]
pub fn http_server_listen_with_ssl<F>(
    addr: &str,
    request_fn: F,
    srv_net: &Arc<ServiceNetRs>,
) -> bool
where
    F: Fn(Arc<TcpConn>, http::Request<Vec<u8>>, http::response::Builder) -> ResponseResult
        + Send
        + Sync
        + 'static,
{
    log::info!("http_server_listen_with_ssl: {} ...", addr);

    let conn_fn = |_conn: Arc<TcpConn>| {
        //let hd = _conn.hd;
        //log::info!("[hd={}] conn_fn", hd);
    };

    let close_fn = |_hd: ConnId| {
        //log::info!("[hd={}] close_fn", _hd);
    };

    let (promise, pinky) = PinkySwear::<bool>::new();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let addr2 = addr.to_owned();

    let func = move || {
        //
        let http_server = Arc::new(create_http_server(
            addr2.as_str(),
            conn_fn,
            request_fn,
            close_fn,
            CONNECTION_LIMIT,
            &srv_net2,
        ));

        // listen
        http_server.listen_with_ssl();

        // add tcp server to serivce net
        with_tls_mut!(G_HTTP_SERVER_STORAGE, g, {
            g.http_server_vec.push(http_server);
        });

        //
        pinky.swear(true);
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
        with_tls_mut!(G_HTTP_SERVER_STORAGE, g, {
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

/// Make new http conn with callbacks from http server
#[inline(always)]
pub fn http_server_make_new_conn(http_server: &Arc<HttpServer>, hd: ConnId, sock_addr: SocketAddr) {
    // 运行于 srv_net 线程
    assert!(http_server.srv_net().is_in_service_thread());

    let packet_type = PacketType::Server;
    let request_parser: Arc<ThreadLocal<UnsafeCell<RequestParser>>> = Arc::new(ThreadLocal::new());

    // 连接数量
    if http_server.check_connection_limit() {
        log::error!("connection overflow!!! http_server_make_new_conn failed!!!");
        return;
    }

    // event handler: connect
    let http_server2 = http_server.clone();
    let connection_establish_fn = Box::new(move |conn: Arc<TcpConn>| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());

        http_server2.on_ll_connect(conn);
    });

    // event handler: input( use packet builder to handle input buffer )
    let http_server2 = http_server.clone();
    let connection_read_fn = Box::new(move |conn: Arc<TcpConn>, input_buffer: NetPacketGuard| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());

        http_server2.on_ll_input(conn, input_buffer, &request_parser);
    });

    // event handler: disconnect
    let http_server2 = http_server.clone();
    let connection_lost_fn = Arc::new(move |hd: ConnId| {
        // 运行于 srv_net 线程
        assert!(http_server2.srv_net().is_in_service_thread());
        http_server2.on_ll_disconnect(hd);
    });

    //
    let netctrl = http_server.netctrl().clone();
    let srv_net = http_server.srv_net().clone();

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
