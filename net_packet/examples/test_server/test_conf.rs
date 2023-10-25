//!
//! TestConf
//!

use parking_lot::RwLock;
use std::cell::UnsafeCell;

use commlib::{NodeConf, XmlReader};

thread_local! {
    ///
    pub static G_TEST_CONF: UnsafeCell<TestConf> = UnsafeCell::new(TestConf::new());
}

///
pub struct TestConf {
    pub my: NodeConf,
}

impl TestConf {
    ///
    pub fn new() -> TestConf {
        TestConf {
            my: NodeConf::new(),
        }
    }

    ///
    pub fn init(&mut self, xr: &RwLock<XmlReader>) {
        let xr = xr.read();
        self.my.id = xr.get_u64(vec!["id"], 0);
        self.my.addr = xr.get_string(vec!["addr"], "");
        self.my.port = xr.get_u64(vec!["port"], 0) as u16;
    }
}
