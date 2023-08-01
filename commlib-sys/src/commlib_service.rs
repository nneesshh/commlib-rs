//!
//! Common Library: service
//!

use crossbeam::channel;
use parking_lot::{Condvar, Mutex, RwLock};
use spdlog::get_current_tid;
use std::sync::Arc;

use crate::G_EXIT_CV;

///
#[derive(Debug, Copy, Clone)]
pub enum State {
    Idle = 0,  // 空闲
    Init,      // 初始化
    Start,     // 启动中
    Run,       // 正在运行
    Finishing, // 等待完成
    Finish,    // 已完成，等待关闭
    Closing,   // 关闭中
    Closed,    // 已关闭
    NodeLost,  // 节点丢失（world 管理节点用）
}

pub type ServiceFuncType = dyn FnMut() + Send + Sync + 'static;

/// Service handle
pub struct ServiceHandle {
    pub id: u64,
    pub state: State,

    pub tx: channel::Sender<Box<ServiceFuncType>>,
    pub rx: channel::Receiver<Box<ServiceFuncType>>,

    pub join_handle: Option<std::thread::JoinHandle<()>>,
    pub tid: u64,

    pub clock: crate::Clock,
    pub xml_config: crate::XmlReader,
}

impl ServiceHandle {
    ///
    pub fn new(id: u64, state: State) -> ServiceHandle {
        let (tx, rx) = channel::unbounded::<Box<ServiceFuncType>>();

        Self {
            id,
            state,
            tx,
            rx,
            join_handle: None,
            tid: 0u64,
            clock: crate::Clock::new(),
            xml_config: crate::XmlReader::new(),
        }
    }

    ///
    pub fn id(&self) -> u64 {
        self.id
    }

    ///
    pub fn state(&self) -> State {
        self.state
    }

    ///
    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    ///
    pub fn clock(&self) -> &crate::Clock {
        &self.clock
    }

    ///
    pub fn xml_config(&self) -> &crate::XmlReader {
        &self.xml_config
    }

    ///
    pub fn set_xml_config(&mut self, xml_config: crate::XmlReader) {
        self.xml_config = xml_config;
    }

    ///
    pub fn quit_service(&mut self) {
        self.state = State::Closed;
    }

    /// 在 service 线程中执行回调任务
    pub fn run_in_service(&self, mut cb: Box<dyn FnMut() + Send + Sync + 'static>) {
        if self.is_in_service_thread() {
            cb();
        } else {
            self.tx.send(cb).unwrap();
        }
    }

    /// 当前代码是否运行于 service 线程中
    pub fn is_in_service_thread(&self) -> bool {
        let tid = get_current_tid();
        tid == self.tid
    }

    /// 等待线程结束
    pub fn join_service(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join().unwrap();
        }
    }
}

/// Service start a new single thread, and run callback in it.
pub trait ServiceRs: Send + Sync {
    /// 获取 service nmae
    fn name(&self) -> &str;

    /// 获取 service 句柄
    fn get_handle(&self) -> &RwLock<ServiceHandle>;

    /// 配置 service
    fn conf(&self);

    /// Init in-service
    fn init(&self);

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnMut() + Send + Sync + 'static>);

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool;

    /// 等待线程结束
    fn join(&self);
}

/// 启动 service 线程，service 需要使用 Arc 包装，否则无法跨线程 move
pub fn start_service(srv: &'static dyn ServiceRs, name_of_thread: &str) -> bool {
    let mut handle1_mut = srv.get_handle().write();
    if handle1_mut.tid > 0u64 {
        log::error!("service already started!!! tid={}", handle1_mut.tid);
        return false;
    }

    //
    let tid_cv = Arc::new((Mutex::new(0u64), Condvar::new()));
    let tid_ready = Arc::clone(&tid_cv);

    let exit_cv = Arc::clone(&crate::G_EXIT_CV);

    let tname = name_of_thread.to_owned();
    handle1_mut.join_handle = Some(
        std::thread::Builder::new()
            .name(tname.to_owned())
            .spawn(move || {
                // notify ready
                let tid = get_current_tid();
                let (lock, cvar) = &*tid_ready;
                {
                    // release guard after value ok
                    let mut guard = lock.lock();
                    *guard = tid;
                }
                cvar.notify_all();

                // run
                run_service(srv, tname.as_str());

                // exit
                {
                    let mut handle2_mut = srv.get_handle().write();

                    // mark closed
                    handle2_mut.state = State::Closed;

                    // notify exit
                    (&*exit_cv).1.notify_all();
                    log::info!(
                        "service exit: ID={} state={:?}",
                        handle2_mut.id,
                        handle2_mut.state
                    );
                }
            })
            .unwrap(),
    );

    //tid (ThreadId.as_u64() is not stable yet)
    // wait ready
    let (lock, cvar) = &*tid_cv;
    let mut guard = lock.lock();
    cvar.wait(&mut guard);
    handle1_mut.tid = *guard;
    true
}

///
pub fn run_service(srv: &'static dyn ServiceRs, service_name: &str) {
    // init
    {
        let handle = srv.get_handle().read();
        log::info!("[{}] init ... ID={}", service_name, handle.id);
        srv.init();
    }

    loop {
        let mut run = false;
        {
            let handle = srv.get_handle().read();
            if (State::Closed as u32) == (handle.state as u32) {
                break;
            } else {
                if (State::Closing as u32) == (handle.state as u32) {
                    if handle.rx.is_empty() {
                        // Quit
                        let mut handle_mut = srv.get_handle().write();
                        handle_mut.quit_service();
                        break;
                    }
                    log::debug!("[{}] rx length={}", service_name, handle.rx.len());
                }
                run = true;
            }
        }

        if run {
            let sw = crate::StopWatch::new();

            // handle for write
            {
                let mut handle_mut = srv.get_handle().write();

                // update clock
                handle_mut.clock.update();
            }

            // handle for read
            {
                let handle = srv.get_handle().read();

                // dispatch cb -- process async tasks
                let mut count = 4096_i32;
                while count > 0 && !handle.rx.is_empty() {
                    match handle.rx.try_recv() {
                        Ok(mut cb) => {
                            log::info!("Dequeued item ID={}", handle.id);
                            println!("Dequeued item ID={}", handle.id);
                            cb();
                            count -= 1;
                        }
                        Err(err) => {
                            log::error!("service receive cb error:: {:?}", err);
                        }
                    }
                }

                //
                let cost = sw.elapsed();
                if cost > 60_u128 {
                    log::error!(
                        "[{}] ID={} timeout cost: {}ms",
                        service_name,
                        handle.id,
                        cost
                    );
                }
            }
        }
    }
}
