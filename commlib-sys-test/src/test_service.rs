//!
//! Common Library: service-signal
//!

use app_helper::Startup;
use parking_lot::RwLock;

use commlib_sys::{NodeState, ServerCallbacks, ServiceHandle, ServiceRs};
use commlib_sys::{G_SERVICE_NET, G_SERVICE_SIGNAL};

use std::sync::Arc;

use crate::test_conf::G_TEST_CONF;

pub const SERVICE_ID_TEST_SERVICE: u64 = 10001_u64;
lazy_static::lazy_static! {
    pub static ref G_TEST_SERVICE: Arc<TestService> = Arc::new(TestService::new(SERVICE_ID_TEST_SERVICE));
}

pub struct TestService {
    pub handle: RwLock<ServiceHandle>,
    pub startup: RwLock<Startup>,
}

impl TestService {
    ///
    pub fn new(id: u64) -> TestService {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, NodeState::Idle)),
            startup: RwLock::new(Startup::new(id)),
        }
    }

    fn init_startup(&self) {
        let mut startup_mut = self.startup.write();
        startup_mut.add_step("start network listen", || {
            // TODO: let thread_num: u32 = 1;
            // TODO: let connection_limit: u32 = 0; // 0=no limit

            let mut callbacks = ServerCallbacks::new();
            callbacks.conn_fn = Box::new(|conn_id| {
                log::info!("[conn_fn] conn_id={:?}", conn_id);

                conn_id.send("hello, rust".as_bytes());
            });

            callbacks.msg_fn = Box::new(|conn_id, pkt| {
                log::info!("[msg_fn] conn_id={:?}", conn_id);

                conn_id.send("hello, rust".as_bytes());
            });

            callbacks.stopped_cb = Box::new(|conn_id| {
                log::info!("[stopped_cb] conn_id={:?}", conn_id);

                conn_id.send("bye, rust".as_bytes());
            });

            app_helper::with_conf!(G_TEST_CONF, cfg, {
                G_SERVICE_NET.listen(
                    cfg.my.addr.clone(),
                    cfg.my.port,
                    callbacks,
                    G_TEST_SERVICE.as_ref(),
                );
            });

            //
            true
        });

        //
        startup_mut.run();
    }
}

impl ServiceRs for TestService {
    /// 获取 service nmae
    fn name(&self) -> &str {
        "test_service"
    }

    /// 获取 service 句柄
    fn get_handle(&self) -> &RwLock<ServiceHandle> {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

    /// Init in-service
    fn init(&self) -> bool {
        let mut handle_mut = self.get_handle().write();

        // ctrl-c stop, DEBUG ONLY
        G_SERVICE_SIGNAL.listen_sig_int(G_TEST_SERVICE.as_ref(), || {
            println!("WTF!!!!");
        });
        log::info!("\nGAME init ...\n");

        //
        app_helper::with_conf_mut!(G_TEST_CONF, cfg, { cfg.init(handle_mut.xml_config()) });

        //
        self.init_startup();

        //
        handle_mut.set_state(NodeState::Start);
        true
    }

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send + Sync + 'static>) {
        let handle = self.get_handle().read();
        handle.run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool {
        let handle = self.get_handle().read();
        handle.is_in_service_thread()
    }

    /// 等待线程结束
    fn join(&self) {
        let mut handle_mut = self.get_handle().write();
        handle_mut.join_service();
    }
}
