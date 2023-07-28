#[cxx::bridge]
pub mod ffi_common {

    extern "Rust" {
        pub type ServiceWrapper;
        unsafe fn on_connection(srv: *mut ServiceWrapper);
    }
}

use parking_lot::{Condvar, Mutex, RwLock};
use std::sync::Arc;

pub struct ServiceWrapper {
    srv: Arc<dyn crate::ServiceRs>,
}

unsafe impl ExternType for ServiceWrapper {
    type Id = type_id!("ServiceWrapper");
    type Kind = cxx::kind::Trivial;
}

pub fn on_connection(srv: *mut ServiceWrapper) {}
