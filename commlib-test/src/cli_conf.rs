//!
//! CliConf
//!

use parking_lot::RwLock;
use std::cell::UnsafeCell;

use commlib::{NodeConf, XmlReader};

thread_local! {
    ///
    pub static G_CLI_CONF: UnsafeCell<CliConf> = UnsafeCell::new(CliConf::new());
}

///
pub struct CliConf {
    pub remote: NodeConf,
}

impl CliConf {
    ///
    pub fn new() -> CliConf {
        CliConf {
            remote: NodeConf::new(),
        }
    }

    ///
    pub fn init(&mut self, xr: &RwLock<XmlReader>) {
        let xr = xr.read();
        self.remote.id = xr.get_u64(vec!["id"], 0);
        self.remote.addr = xr.get_string(vec!["addr"], "");
        self.remote.port = xr.get_u64(vec!["port"], 0) as u16;
    }
}
