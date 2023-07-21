//!
//! Common Library: service-signal
//!

use super::commlib_service::*;

pub struct ServiceSignalRs {
    pub handle: ServiceHandle,
}

impl ServiceSignalRs {
    ///
    pub fn new(id: u64, state: State) -> ServiceSignalRs {
        Self {
            handle: ServiceHandle::new(id, state),
        }
    }
}

impl ServiceRs for ServiceSignalRs {
    ///
    fn init(&mut self) {
        let x = 123;
        extern "C" fn on_signal_int(sig: i32) {
            println!("Welcome back in Rust! Value={}", sig);
        }

        extern "C" fn on_signal_usr1(sig: i32) {
            println!("Welcome back in Rust! Value={}", sig);
        }

        extern "C" fn on_signal_usr2(sig: i32) {
            println!("Welcome back in Rust! Value={}", sig);
        }

        let cb1 = crate::SignalCallback(on_signal_int);
        let cb2 = crate::SignalCallback(on_signal_usr1);
        let cb3 = crate::SignalCallback(on_signal_usr2);

        crate::ffi_sig::init_signal_handlers(cb1, cb2, cb3);
    }

    ///
    fn start(&mut self) {
        let rx = self.handle.rx.clone();

        if self.handle.tid.is_some() {
            log::error!("service already started!!! tid={:?}", self.handle.tid);
            return;
        }

        //start thread
        let c = std::thread::Builder::new()
            .name("consumer".to_string())
            .spawn(move || {
                // dispatch cb
                while let Ok(cb) = rx.recv() {
                    //log::info!("Dequeued item");
                    cb();
                }
            })
            .unwrap();

        self.handle.tid = Some(c.thread().id());
    }

    ///
    fn run_in_service(&mut self, cb: fn()) {
        if self.is_in_service_thread() {
            cb();
        } else {
            self.handle.tx.send(Box::new(cb)).unwrap();
        }
    }

    ///
    fn is_in_service_thread(&self) -> bool {
        std::thread::current().id() == self.handle.tid.unwrap()
    }
}
