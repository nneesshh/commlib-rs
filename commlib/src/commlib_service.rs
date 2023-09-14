//!
//! Common Library: service
//!

use bytemuck::NoUninit;
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread::JoinHandle;

use atomic::{Atomic, Ordering};
use crossbeam::channel;
use spdlog::get_current_tid;

use super::G_EXIT_CV;
use super::{Clock, PinkySwear, StopWatch, XmlReader};

#[derive(Debug, PartialEq, PartialOrd, Copy, Clone, NoUninit)]
#[repr(u8)]
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

pub type ServiceFuncType = dyn FnOnce() + Send + Sync; // Note: tait object is always 'static, no need add 'static here

/// Service handle
pub struct ServiceHandle {
    pub id: u64,
    pub state: Atomic<NodeState>,

    pub tx: channel::Sender<Box<ServiceFuncType>>,
    pub rx: channel::Receiver<Box<ServiceFuncType>>,

    pub clock: Clock,

    pub xml_config: RwLock<XmlReader>,

    //
    pub tid: Atomic<u64>,
    pub join_handle_opt: RwLock<Option<JoinHandle<()>>>,
}

impl ServiceHandle {
    ///
    pub fn new(id: u64, state: NodeState) -> ServiceHandle {
        let (tx, rx) = channel::unbounded::<Box<ServiceFuncType>>();

        Self {
            id,
            state: Atomic::new(state),

            tx,
            rx,

            clock: Clock::new(),

            xml_config: RwLock::new(XmlReader::new()),

            tid: Atomic::new(0_u64),
            join_handle_opt: RwLock::new(None),
        }
    }

    ///
    #[inline(always)]
    pub fn id(&self) -> u64 {
        self.id
    }

    ///
    #[inline(always)]
    pub fn state(&self) -> NodeState {
        self.state.load(Ordering::Relaxed)
    }

    ///
    #[inline(always)]
    pub fn set_state(&self, state: NodeState) {
        self.state.store(state, Ordering::Relaxed);
    }

    ///
    #[inline(always)]
    pub fn xml_config(&self) -> &RwLock<XmlReader> {
        &self.xml_config
    }

    ///
    #[inline(always)]
    pub fn set_xml_config(&self, xml_config: XmlReader) {
        let mut xml_config_mut = self.xml_config.write();
        (*xml_config_mut) = xml_config;
    }

    ///
    #[inline(always)]
    pub fn tid(&self) -> u64 {
        self.tid.load(Ordering::Relaxed)
    }

    ///
    #[inline(always)]
    pub fn set_tid(&self, tid: u64) {
        self.tid.store(tid, Ordering::Relaxed);
    }

    /// 在 service 线程中执行回调任务
    #[inline(always)]
    pub fn run_in_service(&self, cb: Box<dyn FnOnce() + Send + Sync>) {
        self.tx.send(cb).unwrap();
    }

    /// 当前代码是否运行于 service 线程中
    #[inline(always)]
    pub fn is_in_service_thread(&self) -> bool {
        let tid = get_current_tid();
        self.tid() == tid
    }

    /// 发送 close 信号
    pub fn quit_service(&self) {
        if self.state() < NodeState::Closed {
            self.set_state(NodeState::Closed);
        }
    }

    /// 等待线程结束
    pub fn join_service(&self) {
        let mut join_handle_opt_mut = self.join_handle_opt.write();
        if let Some(join_handle) = join_handle_opt_mut.take() {
            join_handle.join().unwrap();
        }
    }
}

/// Service start a new single thread, and run callback in it.
pub trait ServiceRs: Send + Sync {
    /// 获取 service nmae
    fn name(&self) -> &str;

    /// 获取 service 句柄
    fn get_handle(&self) -> &ServiceHandle;

    /// 配置 service
    fn conf(&self);

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send + Sync>);

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool;

    /// 等待线程结束
    fn join(&self);
}

/// 启动 service 线程，service 需要使用 Arc 包装，否则无法跨线程 move
pub fn start_service<T, I>(
    srv: &Arc<T>,
    name_of_thread: &str,
    initializer: I,
) -> (Option<JoinHandle<()>>, u64)
where
    T: ServiceRs + 'static,
    I: FnOnce() + Send + Sync + 'static,
{
    let (tid_prms, tid_pinky) = PinkySwear::<u64>::new();

    {
        let handle = srv.get_handle();
        let tid = handle.tid();
        if tid > 0u64 {
            log::error!("service already started!!! tid={}", tid);
            return (None, 0);
        }
    }

    //
    let srv2 = srv.clone();
    let exit_cv = Arc::clone(&G_EXIT_CV);

    let tname = name_of_thread.to_owned();
    let join_handle = std::thread::Builder::new()
        .name(tname.to_owned())
        .spawn(move || {
            let handle = srv2.get_handle();

            // notify ready
            let tid = get_current_tid();
            log::info!("service({}) spawn on thread: {}", tname, tid);

            // 服务线程初始化
            (initializer)();

            // 服务线程就绪通知
            tid_pinky.swear(tid);

            // run
            run_service(srv2.as_ref(), tname.as_str());

            // exit
            {
                // mark closed
                handle.set_state(NodeState::Closed);

                // notify exit
                (&*exit_cv).1.notify_all();
                log::info!("service exit: ID={} state={:?}", handle.id, handle.state());
            }
        })
        .unwrap();

    //
    let tid = tid_prms.wait();
    (Some(join_handle), tid)
}

/// 线程启动完成，执行后续处理 (ThreadId.as_u64() is not stable yet, use ready_pair now)
///    ready_pair: (join_handle_opt, tid)
pub fn proc_service_ready(srv: &dyn ServiceRs, ready_pair: (Option<JoinHandle<()>>, u64)) -> bool {
    let (join_handle_opt, tid) = ready_pair;

    if join_handle_opt.is_some() {
        // update tid
        let handle = srv.get_handle();
        handle.set_tid(tid);

        // update join_handle
        {
            let mut join_handle_opt_mut = handle.join_handle_opt.write();
            (*join_handle_opt_mut) = join_handle_opt;
        }
        true
    } else {
        log::error!("[proc_service_ready] failed!!! tid: {}", tid);
        false
    }
}

fn run_service(srv: &dyn ServiceRs, service_name: &str) {
    let handle = srv.get_handle();
    log::info!("[{}] run ... ID={}", service_name, handle.id);

    // loop until "NodeState::Closed"
    let mut sw = StopWatch::new();
    loop {
        // check run
        let run = {
            let state = handle.state();
            if NodeState::Closed == state {
                false
            } else if NodeState::Closing == state {
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
            // mark service quit
            handle.quit_service();
            break;
        } else {
            // update clock
            Clock::update();

            // dispatch cb -- process async tasks
            let mut count = 4096_i32;
            while count > 0 && !handle.rx.is_empty() {
                match handle.rx.try_recv() {
                    Ok(cb) => {
                        /*log::info!("Dequeued item ID={}", handle.id);
                        println!("Dequeued item ID={}", handle.id);*/
                        cb();
                        count -= 1;
                    }
                    Err(err) => {
                        log::error!("service receive cb error: {:?}", err);
                    }
                }
            }

            // sleep by cost
            let cost = sw.elapsed_and_reset();
            if cost > 60_u128 {
                /*log::error!(
                    "[{}] ID={} timeout cost: {}ms",
                    service_name,
                    handle.id,
                    cost
                );*/
            } else {
                // sleep
                const SLEEP_MS: std::time::Duration = std::time::Duration::from_millis(1);
                std::thread::sleep(SLEEP_MS);
            }
        }
    }
}
