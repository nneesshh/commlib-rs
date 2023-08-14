//!
//! Common Library: service
//!

use crossbeam::channel;
use parking_lot::{Condvar, Mutex, RwLock};
use spdlog::get_current_tid;
use std::sync::Arc;
use std::thread::JoinHandle;

///
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum NodeState {
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
    pub state: NodeState,

    pub tx: channel::Sender<Box<ServiceFuncType>>,
    pub rx: channel::Receiver<Box<ServiceFuncType>>,

    pub tid: u64,
    join_handle_opt: Option<JoinHandle<()>>,

    pub clock: crate::Clock,
    pub xml_config: crate::XmlReader,
}

impl ServiceHandle {
    ///
    pub fn new(id: u64, state: NodeState) -> ServiceHandle {
        let (tx, rx) = channel::unbounded::<Box<ServiceFuncType>>();

        Self {
            id,
            state,
            tx,
            rx,
            tid: 0u64,
            join_handle_opt: None,
            clock: crate::Clock::new(),
            xml_config: crate::XmlReader::new(),
        }
    }

    ///
    pub fn id(&self) -> u64 {
        self.id
    }

    ///
    pub fn state(&self) -> NodeState {
        self.state
    }

    ///
    pub fn set_state(&mut self, state: NodeState) {
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
        if (self.state as u32) < (NodeState::Closed as u32) {
            self.state = NodeState::Closed;
        }
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
        if let Some(join_handle) = self.join_handle_opt.take() {
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

    /// Init in-service （修改变量使用内部小粒度的局部锁，不能锁定整个 service）
    fn init(&self) -> bool;

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnMut() + Send + Sync + 'static>);

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool;

    /// 等待线程结束
    fn join(&self);
}

/// 启动 service 线程，service 需要使用 Arc 包装，否则无法跨线程 move
pub fn start_service(
    srv: &'static dyn ServiceRs,
    name_of_thread: &str,
) -> (Option<JoinHandle<()>>, Option<Arc<(Mutex<u64>, Condvar)>>) {
    {
        let handle = srv.get_handle().read();
        if handle.tid > 0u64 {
            log::error!("service already started!!! tid={}", handle.tid);
            return (None, None);
        }
    }

    //
    let tid_cv = Arc::new((Mutex::new(0u64), Condvar::new()));
    let tid_ready = Arc::clone(&tid_cv);

    let exit_cv = Arc::clone(&crate::G_EXIT_CV);

    let tname = name_of_thread.to_owned();
    let join_handle = std::thread::Builder::new()
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

            //
            srv.init();

            // run, 假如 srv 是 mutable borrow, 那么函数使用完毕后将被释放，此时需要返回出来，后续步骤才能使用
            let srv2 = run_service(srv, tname.as_str());

            // exit with srv2
            {
                let mut handle2_mut = srv2.get_handle().write();

                // mark closed
                handle2_mut.state = NodeState::Closed;

                // notify exit
                (&*exit_cv).1.notify_all();
                log::info!(
                    "service exit: ID={} state={:?}",
                    handle2_mut.id,
                    handle2_mut.state
                );
            }
        })
        .unwrap();

    //
    (Some(join_handle), Some(tid_cv))
}

/// 等待线程启动完成
pub fn wait_service_ready(
    srv: &'static dyn ServiceRs,
    ready_pair: (Option<JoinHandle<()>>, Option<Arc<(Mutex<u64>, Condvar)>>),
) -> bool {
    if let (join_handle_opt, Some(tid_cv)) = ready_pair {
        let mut handle_mut = srv.get_handle().write();
        handle_mut.join_handle_opt = join_handle_opt;

        //tid (ThreadId.as_u64() is not stable yet)
        // wait ready
        let (lock, cvar) = &*tid_cv;
        let mut guard = lock.lock();
        cvar.wait(&mut guard);
        handle_mut.tid = *guard;
        true
    } else {
        log::error!("[wait_service_ready] failed!!!");
        false
    }
}

///
pub fn run_service(srv: &'static dyn ServiceRs, service_name: &str) -> &'static dyn ServiceRs {
    // init
    {
        let handle = srv.get_handle().read();
        log::info!("[{}] init ... ID={}", service_name, handle.id);
    }

    //
    let mut sw = crate::StopWatch::new();
    loop {
        // check run
        let run = {
            let handle = srv.get_handle().read();
            if (NodeState::Closed as u32) == (handle.state as u32) {
                false
            } else if (NodeState::Closing as u32) == (handle.state as u32) {
                if handle.rx.is_empty() {
                    false
                } else {
                    log::debug!("[{}] rx length={}", service_name, handle.rx.len());
                    true
                }
            } else {
                true
            }
        };

        // run or quit ?
        if !run {
            // handle for write
            {
                let mut handle_mut = srv.get_handle().write();

                // mark service quit
                handle_mut.quit_service();
            }
            break;
        } else {
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

                // sleep by cost
                let cost = sw.elapsed_and_reset();
                if cost > 60_u128 {
                    log::error!(
                        "[{}] ID={} timeout cost: {}ms",
                        service_name,
                        handle.id,
                        cost
                    );
                } else {
                    // sleep
                    const SLEEP_MS: std::time::Duration = std::time::Duration::from_millis(1);
                    std::thread::sleep(SLEEP_MS);
                }
            }
        }
    }

    // mutable borrow of service 使用完毕，返回出去供后续步骤使用
    srv
}
