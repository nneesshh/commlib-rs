//!
//! Common Library: service-signal
//!

use spdlog::get_current_tid;
use std::sync::{Arc, Condvar, Mutex};

use super::commlib_service::*;
use crate::commlib_event::*;

/// Event
pub struct EventSignalInt {
    code: u32,
}
crate::impl_event_for!(EventSignalInt);

pub struct EventSignalUsr1 {
    code: u32,
}
crate::impl_event_for!(EventSignalUsr1);

pub struct EventSignalUsr2 {
    code: u32,
}
crate::impl_event_for!(EventSignalUsr2);

/// ServiceSignal
pub struct ServiceSignalRs {
    pub handle: ServiceHandle,
}

impl ServiceSignalRs {
    ///
    pub fn new(id: u64) -> ServiceSignalRs {
        Self {
            handle: ServiceHandle::new(id, State::Idle),
        }
    }

    /// Event: sig_int
    pub fn on_sig_int(&mut self) {
        // Trigger event
        let mut e = EventSignalInt { code: 0 };
        e.trigger();
    }

    /// Event: sig_usr1
    pub fn on_sig_usr1(&mut self) {
        // Trigger event
        let mut e = EventSignalUsr1 { code: 0 };
        e.trigger();
    }

    /// Event: sig_usr2
    pub fn on_sig_usr2(&mut self) {
        // Trigger event
        let mut e = EventSignalUsr2 { code: 0 };
        e.trigger();
    }

    /// Listen signal: sig_int
    pub fn listen_sig_int<'a, F, S>(&mut self, f: F, srv: &'static S)
    where
        F: FnMut() + Send + Sync + 'static,
        S: ServiceRs,
    {
        //let mut f = Some(f);

        let mut f = Some(f);
        EventSignalUsr2::add_callback(move |e| {
            let mut f = f.take();
            srv.run_in_service(Box::new(move || {
                let mut f = f.take().unwrap();
                f();
            }));
        });
    }
}

impl ServiceRs for ServiceSignalRs {
    /// 获取 service 句柄
    fn get_handle(&mut self) -> &mut ServiceHandle {
        &mut self.handle
    }

    /// 配置 service
    fn conf(&mut self) {
        let x = 123;
        extern "C" fn on_signal_int(sig: i32) {
            println!("Recive int signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_int
            let srv = &crate::globals::G_SERVICE_SIGNAL;
            let cb = Box::new(|| srv.lock().unwrap().on_sig_int());
            srv.lock().unwrap().run_in_service(cb);
        }

        extern "C" fn on_signal_usr1(sig: i32) {
            println!("Recive usr1 signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_usr1
            let srv = &crate::globals::G_SERVICE_SIGNAL;
            let cb = Box::new(|| srv.lock().unwrap().on_sig_usr1());
            srv.lock().unwrap().run_in_service(cb);
        }

        extern "C" fn on_signal_usr2(sig: i32) {
            println!("Recive usr2 signal in Rust! Value={}", sig);

            // Post event callback to service thread: sig_usr2
            let srv = &crate::globals::G_SERVICE_SIGNAL;
            let cb = Box::new(|| srv.lock().unwrap().on_sig_usr2());
            srv.lock().unwrap().run_in_service(cb);
        }

        let cb1 = crate::SignalCallback(on_signal_int);
        let cb2 = crate::SignalCallback(on_signal_usr1);
        let cb3 = crate::SignalCallback(on_signal_usr2);

        crate::ffi_sig::init_signal_handlers(cb1, cb2, cb3);
    }

    /// 启动 service 线程
    fn start(&mut self) {
        let rx = self.handle.rx.clone();

        if self.handle.tid > 0u64 {
            log::error!("service already started!!! tid={}", self.handle.tid);
            return;
        }

        //start thread
        let tname = "service_signal".to_owned();
        let tid_cv = Arc::new((Mutex::new(0u64), Condvar::new()));
        let tid_ready = Arc::clone(&tid_cv);
        self.handle.join_handle = Some(
            std::thread::Builder::new()
                .name(tname)
                .spawn(move || {
                    // notify ready
                    let tid = get_current_tid();
                    let (lock, cvar) = &*tid_ready;
                    {
                        // release guard after value ok
                        let mut guard = lock.lock().unwrap();
                        *guard = tid;
                    }
                    cvar.notify_all();

                    // dispatch cb
                    while let Ok(mut cb) = rx.recv() {
                        //log::info!("Dequeued item");
                        cb();
                    }
                })
                .unwrap(),
        );

        //tid (ThreadId.as_u64() is not stable yet)
        // wait ready
        let (lock, cvar) = &*tid_cv;
        let guard = lock.lock().unwrap();
        let tid = cvar.wait(guard).unwrap();
        self.handle.tid = *tid;
    }

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, mut cb: Box<dyn FnMut() + Send + Sync + 'static>) {
        if self.is_in_service_thread() {
            cb();
        } else {
            self.handle.tx.send(cb).unwrap();
        }
    }

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool {
        let tid = get_current_tid();
        tid == self.handle.tid
    }

    /// 等待线程结束
    fn join(&mut self) {
        if let Some(join_handle) = self.handle.join_handle.take() {
            join_handle.join().unwrap();
        }
    }
}
