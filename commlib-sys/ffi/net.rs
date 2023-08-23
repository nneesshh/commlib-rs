#[cxx::bridge]
pub mod ffi_net {

    unsafe extern "C++" {
        include!("net_bindings.h");

        pub type ServiceWrapper = crate::ServiceWrapper;

        #[namespace = "commlib::evpp"]
        type TCPConn;

        #[namespace = "commlib"]
        type NetPacket;

    }

    impl SharedPtr<TCPConn> {}
}
