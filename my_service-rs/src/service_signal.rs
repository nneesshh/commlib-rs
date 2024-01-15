use commlib::{Event, EventHandler, EventListener};
use commlib_sys::ffi_sig::init_signal_handlers;
use commlib_sys::SignalCallback;

use crate::G_SERVICE_SIGNAL;
use crate::{NodeState, ServiceHandle, ServiceRs};
use commlib::impl_event_for;

/// Event
pub struct EventSignalInt();
impl_event_for!("Signal", EventSignalInt);

pub struct EventSignalUsr1();
impl_event_for!("Signal", EventSignalUsr1);

pub struct EventSignalUsr2();
impl_event_for!("Signal", EventSignalUsr2);

/// ServiceSignal
pub struct ServiceSignalRs {
    pub handle: ServiceHandle,
}

impl ServiceSignalRs {
    ///
    pub fn new(id: u64) -> Self {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),
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
    pub fn listen_sig_int<T, F>(&self, srv: &'static T, f: F)
    where
        T: ServiceRs + 'static,
        F: FnOnce() + Send + Sync + 'static,
    {
        self.run_in_service(Box::new(move || {
            let mut f_opt = Some(f);

            // 在 Service thread 中注册事件回调
            EventSignalInt::add_callback(move |_e| {
                // use option trick to take "f" from FnMut() closure
                if let Some(f) = f_opt.take() {
                    // 事件触发时，将 f post 到指定 srv 工作线程中执行
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
    #[inline(always)]
    fn name(&self) -> &str {
        "service_signal"
    }

    /// 获取 service 句柄
    #[inline(always)]
    fn get_handle(&self) -> &ServiceHandle {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {
        extern "C" fn on_signal_int(sig: i32) {
            log::info!("Receive int signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_int
            let cb = Box::new(|| G_SERVICE_SIGNAL.on_sig_int());
            G_SERVICE_SIGNAL.run_in_service(cb);
        }

        extern "C" fn on_signal_usr1(sig: i32) {
            log::info!("Receive usr1 signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_usr1
            let cb = Box::new(|| G_SERVICE_SIGNAL.on_sig_usr1());
            G_SERVICE_SIGNAL.run_in_service(cb);
        }

        extern "C" fn on_signal_usr2(sig: i32) {
            log::info!("Receive usr2 signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_usr2
            let cb = Box::new(|| G_SERVICE_SIGNAL.on_sig_usr2());
            G_SERVICE_SIGNAL.run_in_service(cb);
        }

        let cb1 = SignalCallback(on_signal_int);
        let cb2 = SignalCallback(on_signal_usr1);
        let cb3 = SignalCallback(on_signal_usr2);

        init_signal_handlers(cb1, cb2, cb3);
    }

    /// update
    #[inline(always)]
    fn update(&self) {}

    /// 在 service 线程中执行回调任务
    #[inline(always)]
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send>) {
        self.get_handle().run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    #[inline(always)]
    fn is_in_service_thread(&self) -> bool {
        self.get_handle().is_in_service_thread()
    }

    fn join(&self) {
        self.get_handle().join_service();
    }
}
