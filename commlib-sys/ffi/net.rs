#[cxx::bridge]
pub mod ffi_net {

    extern "Rust" {
        type UserService;
        unsafe fn on_connection(srv: *mut UserService);
    }

    unsafe extern "C++" {
        include!("net_bindings.h");

        #[namespace = "commlib"]
        type ServiceNet = crate::ServiceNetCxx;

        #[namespace = "commlib"]
        type Service;

        #[namespace = "commlib"]
        unsafe fn OnConnection(self: &mut ServiceNet, srv: *mut UserService);

        #[namespace = "commlib"]
        fn service_net_new(n: i32) -> UniquePtr<ServiceNet>;
    }

    impl UniquePtr<ServiceNet> {}
}

#[repr(C)]
pub struct ServiceNetCxx {
    num: i64,
}

unsafe impl cxx::ExternType for ServiceNetCxx {
    type Id = cxx::type_id!("commlib::ServiceNet");
    type Kind = cxx::kind::Trivial;
}

pub struct UserService {
    srv: Box<dyn crate::ServiceRs>,
}

pub fn on_connection(srv: *mut UserService) {}
