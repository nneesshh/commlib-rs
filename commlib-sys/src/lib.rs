//! Commlib: event, log, service, ...

include!("../ffi/signal.rs");
include!("../ffi/net.rs");

///
pub mod app_helper;

///
pub mod commlib_event;

///
pub mod commlib_log;

///
pub mod commlib_service;
pub use commlib_service::{ServiceRs, State};

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

/// Lock for init
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

#[allow(dead_code)]
static INIT: AtomicBool = AtomicBool::new(false);
#[allow(dead_code)]
static INIT_LOCK: Mutex<()> = Mutex::new(());
