//!
//! TestService
//!

use std::sync::Arc;

use commlib::{NodeState, ServiceHandle, ServiceRs};

pub const SERVICE_ID_TEST_SERVICE: u64 = 10001_u64;
lazy_static::lazy_static! {
    pub static ref G_TEST_SERVICE: Arc<TestService> = Arc::new(TestService::new(SERVICE_ID_TEST_SERVICE));
}

pub struct TestService {
    pub handle: ServiceHandle,
}

impl TestService {
    ///
    pub fn new(id: u64) -> Self {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),
        }
    }
}

impl ServiceRs for TestService {
    /// 获取 service name
    #[inline(always)]
    fn name(&self) -> &str {
        "test_service"
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
