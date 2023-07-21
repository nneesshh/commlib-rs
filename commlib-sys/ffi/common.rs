#[cxx::bridge]
pub mod ffi_common {

    extern "Rust" {
        pub type UserService;
        unsafe fn on_connection(srv: *mut UserService);
    }
}

pub struct UserService {
    srv: Box<dyn crate::ServiceRs>,
}

unsafe impl ExternType for UserService {
    type Id = type_id!("UserService");
    type Kind = cxx::kind::Trivial;
}

pub fn on_connection(srv: *mut UserService) {}
