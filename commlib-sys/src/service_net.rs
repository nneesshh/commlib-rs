//!
//! Common Library: service-signal
//!
use parking_lot::RwLock;
//use std::sync::Arc;

use super::commlib_service::*;

pub struct ServiceNetRs {
    pub handle: RwLock<ServiceHandle>,
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64) -> ServiceNetRs {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, State::Idle)),
        }
    }

    ///
    pub fn listen(srv: &dyn ServiceRs, name: &str) {}
}

impl ServiceRs for ServiceNetRs {
    /// 获取 service nmae
    fn name(&self) -> &str {
        "service_net"
    }

    /// 获取 service 句柄
    fn get_handle(&self) -> &RwLock<ServiceHandle> {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

    /// Init in-service
    fn init(&self) {}

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
