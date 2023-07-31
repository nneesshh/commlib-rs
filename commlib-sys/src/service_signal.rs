//!
//! Common Library: service-signal
//!
use parking_lot::{Condvar, Mutex, RwLock};
use spdlog::get_current_tid;
use std::{borrow::BorrowMut, sync::Arc};

use super::commlib_service::*;
use crate::commlib_event::*;

/// Event
pub struct EventSignalInt {
    code: u32,
}
crate::impl_event_for!(ServiceSignalRs, EventSignalInt);

pub struct EventSignalUsr1 {
    code: u32,
}
crate::impl_event_for!(ServiceSignalRs, EventSignalUsr1);

pub struct EventSignalUsr2 {
    code: u32,
}
crate::impl_event_for!(ServiceSignalRs, EventSignalUsr2);

/// ServiceSignal
pub struct ServiceSignalRs {
    pub handle: RwLock<ServiceHandle>,
}

impl ServiceSignalRs {
    ///
    pub fn new(id: u64) -> ServiceSignalRs {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, State::Idle)),
        }
    }

    /// Event: sig_int
    pub fn on_sig_int(&self) {
        // Trigger event
        let mut e = EventSignalInt { code: 0 };
        e.trigger();
    }

    /// Event: sig_usr1
    pub fn on_sig_usr1(&self) {
        // Trigger event
        let mut e = EventSignalUsr1 { code: 0 };
        e.trigger();
    }

    /// Event: sig_usr2
    pub fn on_sig_usr2(&self) {
        // Trigger event
        let mut e = EventSignalUsr2 { code: 0 };
        e.trigger();
    }

    /// Listen signal: sig_int
    pub fn listen_sig_int<F>(&self, srv:&'static dyn ServiceRs, f: F)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        let mut f = Some(f);

        self.run_in_service(Box::new(move || {
            let mut f = f.take();

            // 在 Service thread 中注册事件回调
            EventSignalInt::add_callback(move |e| {
                let mut f = f.take();

                // 事件触发时，将 f post 到工作线程执行
                srv.run_in_service(Box::new(move || {
                    let mut f = f.take().unwrap();
                    f();
                }));
            });
        }));
    }
}

impl ServiceRs for ServiceSignalRs {
    /// 获取 service nmae
    fn name(&self) -> &str {
        "service_signal"
    }

    /// 获取 service 句柄
    fn get_handle(&self) -> &RwLock<ServiceHandle> {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {
        extern "C" fn on_signal_int(sig: i32) {
            println!("Recive int signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_int
            let srv = &crate::globals::G_SERVICE_SIGNAL;
            let cb = Box::new(|| srv.on_sig_int());
            srv.run_in_service(cb);
        }

        extern "C" fn on_signal_usr1(sig: i32) {
            println!("Recive usr1 signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_usr1
            let srv = &crate::globals::G_SERVICE_SIGNAL;
            let cb = Box::new(|| srv.on_sig_usr1());
            srv.run_in_service(cb);
        }

        extern "C" fn on_signal_usr2(sig: i32) {
            println!("Recive usr2 signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_usr2
            let srv = &crate::globals::G_SERVICE_SIGNAL;
            let cb = Box::new(|| srv.on_sig_usr2());
            srv.run_in_service(cb);
        }

        let cb1 = crate::SignalCallback(on_signal_int);
        let cb2 = crate::SignalCallback(on_signal_usr1);
        let cb3 = crate::SignalCallback(on_signal_usr2);

        crate::ffi_sig::init_signal_handlers(cb1, cb2, cb3);
    }

    /// Init in-service
    fn init(&self) {}

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnMut() + Send + Sync + 'static>) {
        let handle = self.get_handle().read();
        handle.run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool {
        let handle = self.get_handle().read();
        handle.is_in_service_thread()
    }

    fn join(&self) {
        let mut handle_mut = self.get_handle().write();
        handle_mut.join_service();
    }
}
