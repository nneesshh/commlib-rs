//!
//! SimpleService
//!

use std::sync::Arc;

use commlib::{http_server_listen, G_SERVICE_NET};
use commlib::{ConnId, NodeState, ServiceHandle, ServiceRs, TcpConn};

use app_helper::conf::Conf;
use app_helper::G_CONF;

///
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
pub fn test_http_server(_conf: &Arc<Conf>) {
    // pre-startup, main manager init
    commlib::ossl_init();

    //
    let srv: &Arc<SimpleService> = &G_SIMPLE_SERVICE;

    //
    let g_conf = G_CONF.load();

    let request_fn = |conn: Arc<TcpConn>,
                      req: http::Request<Vec<u8>>,
                      response_builder: http::response::Builder| {
        let hd = conn.hd;
        log::info!("[hd={}] request_fn", hd);

        let req_body_vec = req.body();
        let req_body = unsafe { std::str::from_utf8_unchecked(req_body_vec.as_slice()) };
        println!("req_body: {}", req_body);

        let rand_pass = commlib::gen_password(10);
        let msg = std::format!("hello simple service, rand_pass={}", rand_pass);
        let resp_body_vec = msg.as_bytes().to_vec();

        //
        let response = response_builder.body(resp_body_vec).unwrap();
        Ok(response)
    };

    http_server_listen(
        srv,
        "127.0.0.1",
        g_conf.http_port,
        request_fn,
        &G_SERVICE_NET,
    );
}
