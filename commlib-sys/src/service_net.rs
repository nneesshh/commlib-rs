//!
//! Common Library: service-signal
//!

use super::commlib_service::*;

pub struct ServiceNetRs {
    pub handle: ServiceHandle,
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64, state: State) -> ServiceNetRs {
        Self {
            handle: ServiceHandle::new(id, state),
        }
    }
}

impl ServiceRs for ServiceNetRs {
    /// 获取 service 句柄
    fn get_handle(&self)->&ServiceHandle {
        &self.handle
    }

    /// 初始化 service
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

    /// 启动 service 线程
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

    /// 在 service 线程中执行回调任务
    fn run_in_service(&mut self, cb: Box<dyn FnOnce() + Send + Sync>) {
        if self.is_in_service_thread() {
            cb();
        } else {
            self.handle.tx.send(cb).unwrap();
        }
    }

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool {
        std::thread::current().id() == self.handle.tid.unwrap()
    }
}
