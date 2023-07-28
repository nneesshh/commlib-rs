//!
//! Common Library: service-signal
//!

use spdlog::get_current_tid;
use std::sync::{Arc, Condvar, Mutex, RwLock};

use commlib_sys::*;

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
    /// 获取 service 句柄
    fn get_handle(&self) -> &RwLock<ServiceHandle> {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnMut() + Send + Sync + 'static>) {
        let handle = self.get_handle().read().unwrap();
        handle.run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool {
        let handle = self.get_handle().read().unwrap();
        handle.is_in_service_thread()
    }

    /// 等待线程结束
    fn join(&self) {
        let mut handle_mut = self.get_handle().write().unwrap();
        handle_mut.join_service();
    }

}
