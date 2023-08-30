//! Commlib: event, log, service, ...

include!("../ffi/common.rs");
include!("../ffi/signal.rs");
include!("../ffi/net.rs");

///
pub mod base64_util;
pub use base64_util::Base64;

///
pub mod stop_watch;
pub use stop_watch::*;

///
pub mod rand_util;
pub use rand_util::*;

///
pub mod string_util;
pub use string_util::*;

/// util for promise blocking wait
pub mod pinky_swear;
pub use pinky_swear::{Pinky, PinkySwear};

///
pub mod commlib_event;
pub use commlib_event::*;

///
pub mod commlib_log;
pub use commlib_log::*;

///
pub mod commlib_service;
pub use commlib_service::*;

///
pub mod clock;
pub use clock::*;

///
pub mod hash_wheel_timer;

///
pub mod xmlreader;
pub use xmlreader::XmlReader;

///
pub mod service_signal;
pub use service_signal::ServiceSignalRs;

///
pub mod service_net;
pub use service_net::{
    connect_to_tcp_server, create_tcp_client, listen_tcp_addr, start_network, stop_network,
};
pub use service_net::{
    CmdId, ConnId, NetPacket, NetPacketGuard, NetProxy, PacketType, ServiceNetRs, TcpClient,
    TcpHandler, TcpListenerId, TcpServer,
};
pub use service_net::{ENCRYPT_KEY_LEN, ENCRYPT_MAX_LEN};

/// 全局变量
pub mod globals;
pub use globals::*;

/// 通用定义
pub mod commlib_def;
pub use commlib_def::*;
