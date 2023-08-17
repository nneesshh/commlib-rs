//!
//! TestManager
//!

use commlib_sys::service_net::PacketType;
use commlib_sys::{NetProxy, ServerCallbacks, ServiceRs};

thread_local! {
    ///
    pub static G_TEST_MANAGER: std::cell::RefCell<TestManager> = {
        std::cell::RefCell::new(TestManager::new())
    };
}

///
pub struct TestManager {
    pub server_proxy: NetProxy,
}

impl TestManager {
    ///
    pub fn new() -> TestManager {
        TestManager {
            server_proxy: NetProxy::new(PacketType::Server),
        }
    }
}
