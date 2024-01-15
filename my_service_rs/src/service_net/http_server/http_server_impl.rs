//!
//! Commlib: HttpServer
//!

use atomic::{Atomic, Ordering};
use std::cell::UnsafeCell;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thread_local::ThreadLocal;

use commlib::G_THREAD_POOL;
use net_packet::NetPacketGuard;

use crate::service_net::http_server::http_server_manager::http_server_make_new_conn;
use crate::service_net::listener::Listener;
use crate::service_net::low_level_network::MessageIoNetwork;
use crate::{ConnId, ServerStatus, ServiceNetRs, ServiceRs, TcpConn};

use super::error;
use super::request_parser::RequestParser;
use super::response_writer::write_response;

const STATIC_DIR: Option<&str> = Some("static");

pub type ResponseResult = Result<http::Response<Vec<u8>>, error::Error>;

///
pub struct HttpServer {
    //
    status: Atomic<ServerStatus>,

    //
    pub addr: String,

    //
    connection_limit: usize,
    connection_num: Atomic<usize>,

    //
    netctrl: Arc<MessageIoNetwork>,
    srv_net: Arc<ServiceNetRs>,

    //
    conn_fn: Arc<dyn Fn(Arc<TcpConn>) + Send + Sync>,
    request_fn: Arc<
        dyn Fn(Arc<TcpConn>, http::Request<Vec<u8>>, http::response::Builder) -> ResponseResult
            + Send
            + Sync,
    >,
    close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,
}

impl HttpServer {
    ///
    pub fn new<C, R, S>(
        addr: &str,
        conn_fn: C,
        request_fn: R,
        close_fn: S,
        connection_limit: usize,
        netctrl: &Arc<MessageIoNetwork>,
        srv_net: &Arc<ServiceNetRs>,
    ) -> Self
    where
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        R: Fn(Arc<TcpConn>, http::Request<Vec<u8>>, http::response::Builder) -> ResponseResult
            + Send
            + Sync
            + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        Self {
            status: Atomic::new(ServerStatus::Null),

            addr: addr.to_owned(),

            connection_limit,
            connection_num: Atomic::new(0_usize),

            netctrl: netctrl.clone(),
            srv_net: srv_net.clone(),

            conn_fn: Arc::new(conn_fn),
            request_fn: Arc::new(request_fn),
            close_fn: Arc::new(close_fn),
        }
    }

    ///
    #[inline(always)]
    pub fn on_ll_connect(self: &Arc<Self>, conn: Arc<TcpConn>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // 连接计数 + 1
        let old_connection_num = self.connection_num.fetch_add(1, Ordering::Relaxed);
        assert!(old_connection_num < 100000000);

        //
        /*log::info!(
            "[on_ll_connect][hd={}] connection_limit({}) connection_num: {} -> {}",
            conn.hd,
            self.connection_limit,
            old_connection_num,
            self.connection_num()
        );*/

        // post 到线程池（根据 ConnId 绑定线程）
        let conn_fn = self.conn_fn.clone();
        G_THREAD_POOL.execute(conn.hd.id, move || {
            //
            (*conn_fn)(conn);
        });
    }

    ///
    #[inline(always)]
    pub fn on_ll_input(
        self: &Arc<Self>,
        conn: Arc<TcpConn>,
        input_buffer: NetPacketGuard,
        request_parser: &Arc<ThreadLocal<UnsafeCell<RequestParser>>>,
    ) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // post 到线程池（根据 ConnId 绑定线程）
        let http_server2 = self.clone();
        let request_parser2 = request_parser.clone();
        G_THREAD_POOL.execute(conn.hd.id, move || {
            //
            let parser = get_request_parser(&request_parser2, &http_server2);
            parser.parse(&conn, input_buffer);
        });
    }

    ///
    #[inline(always)]
    pub fn on_ll_disconnect(self: &Arc<Self>, hd: ConnId) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // post 到线程池（根据 ConnId 绑定线程）
        let close_fn = self.close_fn.clone();
        G_THREAD_POOL.execute(hd.id, move || {
            //
            (*close_fn)(hd);
        });

        // 连接计数 - 1
        let old_connection_num = self.connection_num.fetch_sub(1, Ordering::Relaxed);
        assert!(old_connection_num < 100000000);

        //
        /*log::info!(
            "[on_ll_disconnect][hd={}] connection_limit({}) connection_num: {} -> {}",
            hd,
            self.connection_limit,
            old_connection_num,
            self.connection_num()
        );*/
    }

    /// Create a http server and listen on [ip:port]
    pub fn listen(self: &Arc<Self>) {
        log::info!(
            "http server start listen at addr: {} ... status: {}",
            self.addr,
            self.status().to_string()
        );

        let listener = self.create_listener();
        listener.listen_with_tcp(self.addr.as_str());
    }

    /// Create a http server and listen on [ip:port]
    #[cfg(feature = "ssl")]
    pub fn listen_with_ssl(
        self: &Arc<Self>,
        cert_path_opt: Option<String>,
        private_key_path_opt: Option<String>,
    ) {
        log::info!(
            "http server start listen_with_ssl at addr: {} ... status: {}",
            self.addr,
            self.status().to_string()
        );

        //
        let listener = self.create_listener();

        //
        let cert_ptah = cert_path_opt.unwrap_or("certificate/myserver.pem".to_owned());
        let pri_key_path = private_key_path_opt.unwrap_or("certificate/myserver.key".to_owned());
        listener.listen_with_ssl(
            self.addr.as_str(),
            cert_ptah.as_str(),
            pri_key_path.as_str(),
        );
    }

    ///
    pub fn stop(&self) {
        // TODO:
    }

    ///
    #[inline(always)]
    pub fn connection_num(&self) -> usize {
        self.connection_num.load(Ordering::Relaxed)
    }

    ///
    #[inline(always)]
    pub fn status(&self) -> ServerStatus {
        self.status.load(Ordering::Relaxed)
    }

    ///
    #[inline(always)]
    pub fn set_status(&self, status: ServerStatus) {
        self.status.store(status, Ordering::Relaxed);
    }

    ///
    #[inline(always)]
    pub fn netctrl(&self) -> &Arc<MessageIoNetwork> {
        &self.netctrl
    }

    ///
    #[inline(always)]
    pub fn srv_net(&self) -> &Arc<ServiceNetRs> {
        &self.srv_net
    }

    ///
    #[inline(always)]
    pub fn request_fn_clone(
        &self,
    ) -> Arc<
        dyn Fn(Arc<TcpConn>, http::Request<Vec<u8>>, http::response::Builder) -> ResponseResult
            + Send
            + Sync,
    > {
        //
        self.request_fn.clone()
    }

    ///
    #[inline(always)]
    pub fn check_connection_limit(&self) -> bool {
        // check 连接数上限 (0 == self.connection_limit 代表无限制)
        if self.connection_limit > 0 {
            let connection_num = self.connection_num();
            if connection_num >= self.connection_limit {
                //
                log::error!(
                    "**** **** [check_connection_limit()] connection_limit({}/{}) reached!!! **** ****",
                    self.connection_limit,
                    connection_num
                );

                // 已经达到上限
                true
            } else {
                // 未达到上限
                false
            }
        } else {
            // 无需检测上限 == 未达到上限
            false
        }
    }

    fn create_listener(self: &Arc<Self>) -> Arc<Listener> {
        //
        self.set_status(ServerStatus::Starting);

        //
        let http_server = self.clone();
        let listen_fn = move |r| {
            match r {
                Ok((_listener_id, _sock_addr)) => {
                    // 状态：Running
                    http_server.set_status(ServerStatus::Running);
                }
                Err(error) => {
                    //
                    log::error!(
                        "http server listen at {:?} failed!!! status:{}!!! error: {}!!!",
                        http_server.addr,
                        http_server.status().to_string(),
                        error
                    );
                }
            }
        };

        //
        let http_server2 = self.clone();
        let accept_fn = move |_listener_id, hd, sock_addr| {
            // make new connection
            http_server_make_new_conn(&http_server2, hd, sock_addr);
        };

        // start listener
        let listener = Arc::new(Listener::new(
            std::format!("listener({})", self.addr).as_str(),
            listen_fn,
            accept_fn,
            &self.netctrl,
            &self.srv_net,
        ));
        listener
    }
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

#[inline(always)]
fn get_request_parser<'a>(
    tls_request_parser: &'a Arc<ThreadLocal<UnsafeCell<RequestParser>>>,
    http_server: &Arc<HttpServer>,
) -> &'a mut RequestParser {
    //
    let parser = tls_request_parser.get_or(|| {
        let request_fn = http_server.request_fn_clone();

        // debug only
        /*log::info!(
            "tls_request_parser ptr={:p}",
            (tls_request_parser as *const Arc<ThreadLocal<UnsafeCell<RequestParser>>>)
        );*/

        let parse_cb = move |conn: Arc<TcpConn>, req: http::Request<Vec<u8>>| {
            //
            let request_fn2 = request_fn.clone();
            match process_http_request(req, request_fn2, conn) {
                Ok(_) => {
                    // do nothing
                }
                Err(e) => {
                    //
                    log::error!("get_request_parser error: {:?}", e);
                }
            }
        };
        UnsafeCell::new(RequestParser::new(Box::new(parse_cb)))
    });
    unsafe { &mut *(parser.get()) }
}
