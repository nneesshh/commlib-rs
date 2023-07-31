#[cxx::bridge]
pub mod ffi_net {

    unsafe extern "C++" {
        include!("net_bindings.h");

        pub type TcpCallbacks = crate::TcpCallbacks;
        pub type ServiceWrapper = crate::ServiceWrapper;

        #[namespace = "commlib::evpp"]
        type TCPConn;

        #[namespace = "commlib"]
        type NetPacket;

        #[namespace = "commlib"]
        unsafe fn connect_to_tcp_server(
            srv: *mut ServiceWrapper,
            srvNet: *mut ServiceWrapper,
            name: String,
            addr: String,
            handler: TcpCallbacks,
        );
    }

    impl SharedPtr<TCPConn> {}
}

#[repr(C)]
pub struct TcpCallbacks {
    pub on_listen: extern "C" fn(&ServiceWrapper, String),
    pub on_accept: extern "C" fn(&ServiceWrapper, cxx::SharedPtr<ffi_net::TCPConn>),
    pub on_encrypt: extern "C" fn(&ServiceWrapper, cxx::SharedPtr<ffi_net::TCPConn>),

    pub on_connect: extern "C" fn(&ServiceWrapper, cxx::SharedPtr<ffi_net::TCPConn>),
    pub on_packet: unsafe extern "C" fn(
        &ServiceWrapper,
        cxx::SharedPtr<ffi_net::TCPConn>,
        *mut ffi_net::NetPacket,
    ),
    pub on_close: extern "C" fn(&ServiceWrapper, cxx::SharedPtr<ffi_net::TCPConn>),
}

unsafe impl cxx::ExternType for TcpCallbacks {
    type Id = cxx::type_id!("TcpCallbacks");
    type Kind = cxx::kind::Trivial;
}
