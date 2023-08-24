//!
//! Common Library: service-signal
//!
use parking_lot::RwLock;

use super::commlib_service::*;
use crate::commlib_event::*;

/// Event
pub struct EventSignalInt();
crate::impl_event_for!(ServiceSignalRs, EventSignalInt);

pub struct EventSignalUsr1();
crate::impl_event_for!(ServiceSignalRs, EventSignalUsr1);

pub struct EventSignalUsr2();
crate::impl_event_for!(ServiceSignalRs, EventSignalUsr2);

/// ServiceSignal
pub struct ServiceSignalRs {
    pub handle: RwLock<ServiceHandle>,
}

impl ServiceSignalRs {
    ///
    pub fn new(id: u64) -> ServiceSignalRs {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, NodeState::Idle)),
        }
    }

    /// Event: sig_int
    pub fn on_sig_int(&self) {
        // Trigger event
        let mut e = EventSignalInt {};
        e.trigger();
    }

    /// Event: sig_usr1
    pub fn on_sig_usr1(&self) {
        // Trigger event
        let mut e = EventSignalUsr1 {};
        e.trigger();
    }

    /// Event: sig_usr2
    pub fn on_sig_usr2(&self) {
        // Trigger event
        let mut e = EventSignalUsr2 {};
        e.trigger();
    }

    /// Listen signal: sig_int
    pub fn listen_sig_int<F>(&self, srv: &'static dyn ServiceRs, f: F)
    where
        F: FnOnce() + Send + Sync + 'static,
    {
        self.run_in_service(Box::new(move || {
            let mut f_opt = Some(f);

            // 在 Service thread 中注册事件回调
            EventSignalInt::add_callback(move |_e| {
                // use option trick to take "f" from FnMut() closure
                if let Some(f) = f_opt.take() {
                    // 事件触发时，将 f post 到工作线程执行
                    srv.run_in_service(Box::new(move || {
                        // Notice: f() only works one time because using option trick, see "let f = f_opt.take()"
                        f();
                    }));
                } else {
                    log::error!("EventSignalInt: can't trigger more than one times!!!");
                }
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
            log::info!("Recive int signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_int
            let srv = &crate::globals::G_SERVICE_SIGNAL;
            let cb = Box::new(|| srv.on_sig_int());
            srv.run_in_service(cb);
        }

        extern "C" fn on_signal_usr1(sig: i32) {
            log::info!("Recive usr1 signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_usr1
            let srv = &crate::globals::G_SERVICE_SIGNAL;
            let cb = Box::new(|| srv.on_sig_usr1());
            srv.run_in_service(cb);
        }

        extern "C" fn on_signal_usr2(sig: i32) {
            log::info!("Recive usr2 signal in Rust! Value={}", sig);

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

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send + Sync>) {
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
