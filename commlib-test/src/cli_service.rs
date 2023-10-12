//!
//! CliService
//!

use std::sync::Arc;

use commlib::{NodeState, ServiceHandle, ServiceRs};

pub const SERVICE_ID_CLI_SERVICE: u64 = 1000_u64;
lazy_static::lazy_static! {
    pub static ref G_CLI_SERVICE: Arc<CliService> = Arc::new(CliService::new(SERVICE_ID_CLI_SERVICE));
}

pub struct CliService {
    pub handle: ServiceHandle,
}

impl CliService {
    ///
    pub fn new(id: u64) -> Self {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),
        }
    }
}

impl ServiceRs for CliService {
    /// 获取 service nmae
    #[inline(always)]
    fn name(&self) -> &str {
        "cli_service"
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
