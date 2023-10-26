use atomic::Atomic;
use parking_lot::RwLock;
use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::fs::File;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thread_local::ThreadLocal;

use net_packet::{take_large_packet, NetPacketGuard};

use crate::service_net::net_packet_encdec::PacketType;
use crate::service_net::service_net_impl::create_http_server;
use crate::service_net::tcp_conn_manager::{insert_connection, on_connection_established};
use crate::service_net::{ConnId, TcpConn};
use crate::{PinkySwear, ServiceNetRs, ServiceRs};

use super::error;
use super::http_server_impl::ResponseResult;
use super::request_parser::RequestParser;
use super::HttpServer;

const CONNECTION_LIMIT: usize = 10000;
const STATIC_DIR: Option<&str> = Some("static");

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
pub fn http_server_listen<T, F>(
    srv: &Arc<T>,
    ip: &str,
    port: u16,
    request_fn: F,
    srv_net: &Arc<ServiceNetRs>,
) -> bool
where
    T: ServiceRs + 'static,
    F: Fn(Arc<TcpConn>, http::Request<Vec<u8>>, http::response::Builder) -> ResponseResult
        + Send
        + Sync
        + 'static,
{
    log::info!("http_server_listen: {}:{}...", ip, port);

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
    let srv2 = srv.clone();
    let addr = std::format!("{}:{}", ip, port);

    let func = move || {
        //
        let http_server = Arc::new(create_http_server(
            &srv2,
            addr.as_str(),
            conn_fn,
            request_fn,
            close_fn,
            CONNECTION_LIMIT,
            &srv_net2,
        ));

        // listen
        http_server.listen(&srv2, |listener_id, sock_addr| {
            //
            log::info!(
                "http_server_listen: listener_id={} sock_addr={} ready.",
                listener_id,
                sock_addr
            );
        });

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
        assert!(conn.srv_net.is_in_service_thread());

        http_server2.on_ll_connect(conn);
    });

    // event handler: input( use packet builder to handle input buffer )
    let http_server2 = http_server.clone();
    let connection_read_fn = Box::new(move |conn: Arc<TcpConn>, input_buffer: NetPacketGuard| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net.is_in_service_thread());

        let parser = get_request_parser(&request_parser, &http_server2);
        http_server2.on_ll_input(conn, input_buffer, parser);
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
    let srv = http_server.srv().clone();

    let conn = Arc::new(TcpConn {
        //
        hd,

        //
        sock_addr,

        //
        packet_type: Atomic::new(packet_type),
        closed: Atomic::new(false),

        //
        srv: srv.clone(),
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
fn get_request_parser<'a>(
    tls_request_parser: &'a Arc<ThreadLocal<UnsafeCell<RequestParser>>>,
    http_server: &Arc<HttpServer>,
) -> &'a mut RequestParser {
    // 运行于 srv_net 线程
    assert!(http_server.srv_net().is_in_service_thread());

    let parser = tls_request_parser.get_or(|| {
        let srv = http_server.srv().clone();
        let request_fn = http_server.request_fn_clone();

        // debug only
        /*log::info!(
            "tls_request_parser ptr={:p}",
            (tls_request_parser as *const Arc<ThreadLocal<UnsafeCell<RequestParser>>>)
        );*/

        let parse_cb = move |conn: Arc<TcpConn>, req: http::Request<Vec<u8>>| {
            // 运行于 srv_net 线程
            assert!(conn.srv_net.is_in_service_thread());

            // post 到指定 srv 工作线程中执行
            let request_fn2 = request_fn.clone();
            srv.run_in_service(Box::new(move || {
                //
                match process_http_request(req, request_fn2, conn) {
                    Ok(_) => {
                        // do nothing
                    }
                    Err(e) => {
                        //
                        log::error!("process_http_request error: {:?}", e);
                    }
                }
            }));
        };
        UnsafeCell::new(RequestParser::new(Box::new(parse_cb)))
    });
    unsafe { &mut *(parser.get()) }
}

#[inline(always)]
fn write_response<T: Borrow<[u8]>>(response: http::Response<T>, conn: &Arc<TcpConn>) {
    let (parts, body) = response.into_parts();
    let body: &[u8] = body.borrow();

    const HEADER_SIZE_MAX: usize = 4096; // should we use 4k header?
    let mut resp_buffer = take_large_packet(HEADER_SIZE_MAX + body.len());

    resp_buffer.append_slice(
        std::format!(
            "HTTP/1.1 {} {}\r\n",
            parts.status.as_str(),
            parts
                .status
                .canonical_reason()
                .expect("Unsupported HTTP Status"),
        )
        .as_bytes(),
    );

    if !parts.headers.contains_key(http::header::DATE) {
        let now = chrono::Utc::now();
        resp_buffer.append_slice(
            std::format!("{}\r\n", now.format("%a, %d %b %Y %H:%M:%S GMT")).as_bytes(),
        );
    }
    if !parts.headers.contains_key(http::header::CONNECTION) {
        resp_buffer.append_slice(b"connection: close\r\n");
    }
    if !parts.headers.contains_key(http::header::CONTENT_LENGTH) {
        resp_buffer.append_slice(std::format!("content-length: {}\r\n", body.len()).as_bytes());
    }
    for (k, v) in parts.headers.iter() {
        match v.to_str() {
            Ok(s) => {
                //
                resp_buffer.append_slice(std::format!("{}: {}\r\n", k.as_str(), s).as_bytes());
            }
            Err(_) => {
                //
            }
        }
    }

    resp_buffer.append_slice(b"\r\n"); // http EOF
    resp_buffer.append_slice(body); // http content

    conn.send(resp_buffer.consume());
    conn.close();
}

#[inline(always)]
fn process_http_request(
    req: http::Request<Vec<u8>>,
    request_fn: Arc<
        dyn Fn(Arc<TcpConn>, http::Request<Vec<u8>>, http::response::Builder) -> ResponseResult
            + Send
            + Sync,
    >,
    conn: Arc<TcpConn>,
) -> Result<(), error::Error> {
    //
    let uri = req.uri();
    let response_builder = http::Response::builder();

    // first, we serve static files
    if let Some(static_dir) = STATIC_DIR {
        let static_path = PathBuf::from(static_dir);
        let fs_path = uri.to_string();

        // the uri always includes a leading /, which means that join will over-write the static directory...
        let fs_path = PathBuf::from(&fs_path[1..]);

        // ... you trying to do something bad?
        let traversal_attempt = fs_path.components().any(|component| match component {
            std::path::Component::Normal(_) => false,
            _ => true,
        });

        if traversal_attempt {
            // GET OUT
            let response = response_builder
                .status(http::StatusCode::NOT_FOUND)
                .body("<h1>404</h1><p>Not found!<p>".as_bytes())
                .unwrap();

            write_response(response, &conn);
            return Ok(());
        }

        let fs_path_real = static_path.join(fs_path);

        if Path::new(&fs_path_real).is_file() {
            let mut f = File::open(&fs_path_real)?;

            let mut source = Vec::new();

            f.read_to_end(&mut source)?;

            let response = response_builder.body(source)?;

            write_response(response, &conn);
            return Ok(());
        }
    }

    // 非静态目录，执行回调函数
    let conn2 = conn.clone();
    match request_fn(conn2, req, response_builder) {
        Ok(response) => {
            write_response(response, &conn);
            Ok(())
        }
        Err(e) => {
            //
            let response_builder = http::Response::builder();
            let err_page = std::format!("<h1>500</h1><p>Internal Server Error: {:?}!<p>", e);

            let response = response_builder
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(err_page.as_bytes())
                .unwrap();

            write_response(response, &conn);
            Ok(())
        }
    }
}
