#[cxx::bridge]
pub mod ffi_common {

    extern "Rust" {
        pub type ServiceWrapper;
    }
}

///
#[repr(C)]
pub struct ServiceWrapper {
    pub srv: String,
}

unsafe impl cxx::ExternType for ServiceWrapper {
    type Id = cxx::type_id!("ServiceWrapper");
    type Kind = cxx::kind::Trivial;
}
