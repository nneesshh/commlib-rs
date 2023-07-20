//! Commlib: event, log, service, ...

include!("ffi_signal.rs");

///
pub mod app_helper;

///
pub mod commlib_event;
///
pub mod commlib_log;
///
pub mod commlib_service;
pub use commlib_service::{Service, State};

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
pub use service_signal::ServiceSignal;

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

#[allow(dead_code)]
static INIT: AtomicBool = AtomicBool::new(false);
#[allow(dead_code)]
static INIT_LOCK: Mutex<()> = Mutex::new(());

#[allow(dead_code)]
struct TrapStack {
    num_traps: usize,
    //trap_thread_data: Option<TrapThreadData>,
    //callbacks: TrapCallbacks,
}

impl TrapStack {
    fn new() -> TrapStack {
        TrapStack {
            num_traps: 0,
            //trap_thread_data: None,
            //callbacks: HashMap::new(),
        }
    }
}

lazy_static::lazy_static! {
    static ref TRAP_STACK: std::sync::Mutex<crate::TrapStack> = std::sync::Mutex::new(crate::TrapStack::new());
    static ref TRAP_OWNER_THREAD_ID: std::thread::ThreadId = std::thread::current().id();
}
