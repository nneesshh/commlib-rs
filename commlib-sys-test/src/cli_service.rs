//!
//! CliService
//!

use app_helper::Startup;
use parking_lot::RwLock;

use commlib_sys::G_SERVICE_SIGNAL;
use commlib_sys::{NodeState, ServiceHandle, ServiceRs};

use std::sync::Arc;

use crate::cli_conf::G_CLI_CONF;

pub const SERVICE_ID_CLI_SERVICE: u64 = 1000_u64;
lazy_static::lazy_static! {
    pub static ref G_CLI_SERVICE: Arc<CliService> = Arc::new(CliService::new(SERVICE_ID_CLI_SERVICE));
}

pub struct CliService {
    pub handle: RwLock<ServiceHandle>,
}

impl CliService {
    ///
    pub fn new(id: u64) -> CliService {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, NodeState::Idle)),
        }
    }
}

impl ServiceRs for CliService {
    /// 获取 service nmae
    fn name(&self) -> &str {
        "cli_service"
    }

    /// 获取 service 句柄
    fn get_handle(&self) -> &RwLock<ServiceHandle> {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

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
