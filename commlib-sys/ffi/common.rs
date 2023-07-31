#[cxx::bridge]
pub mod ffi_common {

    extern "Rust" {
        pub type ServiceWrapper;
    }
}

use parking_lot::{Condvar, Mutex, RwLock};
use std::sync::Arc;

#[repr(transparent)]
pub struct ServiceWrapper {
    srv: &'static dyn ServiceRs,
}

unsafe impl cxx::ExternType for ServiceWrapper {
    type Id = cxx::type_id!("ServiceWrapper");
    type Kind = cxx::kind::Trivial;
}