//!
//! Common Library: service-signal
//!

use parking_lot::RwLock;

use commlib_sys::*;
use std::sync::Arc;

pub const SERVICE_ID_TEST_SERVICE: u64 = 10001_u64;
lazy_static::lazy_static! {
    pub static ref G_TEST_SERVICE: Arc<TestService> = Arc::new(TestService::new(SERVICE_ID_TEST_SERVICE));
}

pub struct TestService {
    pub handle: RwLock<ServiceHandle>,
}

impl TestService {
    ///
    pub fn new(id: u64) -> TestService {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, State::Idle)),
        }
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
    fn init(&self) {
        G_SERVICE_SIGNAL.listen_sig_int(G_TEST_SERVICE.clone(), || {
            println!("WTF!!!!");
        });
    }

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnMut() + Send + Sync + 'static>) {
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
