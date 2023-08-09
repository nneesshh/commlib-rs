//! Commlib: event, log, service, ...

include!("../ffi/common.rs");
include!("../ffi/signal.rs");
include!("../ffi/net.rs");

///
pub mod stop_watch;
pub use stop_watch::*;

///
pub mod rand_util;
pub use rand_util::*;

///
pub mod string_util;
pub use string_util::*;

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
pub use service_net::ServiceNetRs;

///
pub mod globals;
pub use globals::*;

/// 节点 id type
pub type NodeId = u64;

/// 区服 id type
pub type ZoneId = i32;

/// 平台 id type
pub type GroupId = u32;

/// Player id type
pub type PlayerId = u64;
