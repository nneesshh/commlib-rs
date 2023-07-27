#[cxx::bridge]
pub mod ffi_common {

    extern "Rust" {
        pub type ServiceWrapper;
        unsafe fn on_connection(srv: *mut ServiceWrapper);
    }
}

use std::sync::{Arc, Mutex, RwLock};

pub struct ServiceWrapper {
    srv: Arc<RwLock<dyn crate::ServiceRs>>,
}

unsafe impl ExternType for ServiceWrapper {
    type Id = type_id!("ServiceWrapper");
    type Kind = cxx::kind::Trivial;
}

pub fn on_connection(srv: *mut ServiceWrapper) {}
