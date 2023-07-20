#[cxx::bridge]
pub mod ffi {

    extern "C++" {
        include!("service_net.h");

        #[namespace = "commlib"]
        type ServiceNet = crate::ServiceNet;

        #[namespace = "commlib"]
        type Service;

        #[namespace = "commlib"]
        unsafe fn OnConnection(self: &mut ServiceNet, srv: *mut Service);

    }
}

use cxx::{type_id, ExternType};

#[repr(C)]
pub struct ServiceNet{
    num: i64,
}

unsafe impl ExternType for ServiceNet {
    type Id = type_id!("commlib::ServiceNet");
    type Kind = cxx::kind::Trivial;
}