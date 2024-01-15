//! Commlib: event, log, service, ...

///
#[macro_use]
mod macros;

///
pub mod utils;

///
mod stop_watch;
pub use stop_watch::StopWatch;

///
mod commlib_event;
pub use commlib_event::*;

///
mod clock;
pub use clock::*;

///
pub mod hash_wheel_timer;

///
mod xmlreader;
pub use xmlreader::XmlReader;

/// 全局变量
mod globals;
pub use globals::*;

/// 通用定义
mod commlib_def;
pub use commlib_def::*;
