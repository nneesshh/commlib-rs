//!
//! SimpleService
//!

use std::sync::Arc;

use commlib::{http_server_listen, with_tls, G_SERVICE_NET};
use commlib::{ConnId, NetPacketGuard, NodeState, ServiceHandle, ServiceRs, TcpConn};

use app_helper::conf::Conf;
use app_helper::G_CONF;

pub const SERVICE_ID_SIMPLE_SERVICE: u64 = 20001_u64;
lazy_static::lazy_static! {
    pub static ref G_SIMPLE_SERVICE: Arc<SimpleService> = Arc::new(SimpleService::new(SERVICE_ID_SIMPLE_SERVICE));
}

pub struct SimpleService {
    pub handle: ServiceHandle,
}

impl SimpleService {
    ///
    pub fn new(id: u64) -> Self {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),
        }
    }
}

impl ServiceRs for SimpleService {
    /// 获取 service name
    #[inline(always)]
    fn name(&self) -> &str {
        "simple_service"
    }

    /// 获取 service 句柄
    #[inline(always)]
    fn get_handle(&self) -> &ServiceHandle {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

    /// update
    #[inline(always)]
    fn update(&self) {}

    /// 在 service 线程中执行回调任务
    #[inline(always)]
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send + 'static>) {
        self.get_handle().run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    #[inline(always)]
    fn is_in_service_thread(&self) -> bool {
        self.get_handle().is_in_service_thread()
    }

    /// 等待线程结束
    fn join(&self) {
        self.get_handle().join_service();
    }
}

use commlib::G_SERVICE_HTTP_CLIENT;
use serde_json::json;

pub fn test_http_client(srv: &Arc<SimpleService>) {
    let body =
        json!({"foo": false, "bar": null, "answer": 42, "list": [null, "world", true]}).to_string();

    //
    let srv2 = srv.clone();
    G_SERVICE_HTTP_CLIENT.http_post(
        "http://127.0.0.1:7878",
        vec!["Content-Type: application/json".to_owned()],
        body,
        move |code, resp| {
            //
            srv2.run_in_service(Box::new(move || {
                log::info!("hello http code: {}, resp: {}", code, resp);
            }));
        },
    )
}

///
pub fn test_http_server(conf: &Arc<Conf>) {
    // pre-startup, main manager init
    commlib::ossl_init();

    //
    let srv: &Arc<SimpleService> = &G_SIMPLE_SERVICE;

    //
    let g_conf = G_CONF.load();

    let conn_fn = |conn: Arc<TcpConn>| {
        let hd = conn.hd;
        log::info!("[hd={}] conn_fn", hd);
    };

    let pkt_fn = |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
        let hd = conn.hd;
        log::info!("[hd={}] msg_fn", hd);
    };

    let close_fn = |hd: ConnId| {
        log::info!("[hd={}] close_fn", hd);
    };

    let addr = http_server_listen(
        srv,
        "127.0.0.1",
        g_conf.http_port,
        conn_fn,
        pkt_fn,
        close_fn,
        &G_SERVICE_NET,
    );
}
