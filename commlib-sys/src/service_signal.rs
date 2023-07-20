//!
//! Common Library: service-signal
//!

use super::commlib_service::*;

pub struct ServiceSignal {
    pub handle: ServiceHandle,
}

impl ServiceSignal {
    ///
    pub fn new(id: u64, state: State) -> ServiceSignal {
        Self {
            handle: ServiceHandle::new(id, state),
        }
    }
}

impl Service for ServiceSignal {
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
