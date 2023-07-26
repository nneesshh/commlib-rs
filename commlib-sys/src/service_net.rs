//!
//! Common Library: service-signal
//!

use spdlog::get_current_tid;
use std::sync::{Arc, Condvar, Mutex};

use super::commlib_service::*;

pub struct ServiceNetRs {
    pub handle: ServiceHandle,
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64) -> ServiceNetRs {
        Self {
            handle: ServiceHandle::new(id, State::Idle),
        }
    }
}

impl ServiceRs for ServiceNetRs {
    /// 获取 service 句柄
    fn get_handle(&mut self) -> &mut ServiceHandle {
        &mut self.handle
    }

    /// 配置 service
    fn conf(&mut self) {}

    /// 启动 service 线程
    fn start(&mut self) {
        let rx = self.handle.rx.clone();

        if self.handle.tid > 0u64 {
            log::error!("service already started!!! tid={}", self.handle.tid);
            return;
        }

        //
        let tname = "service_net".to_owned();
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
