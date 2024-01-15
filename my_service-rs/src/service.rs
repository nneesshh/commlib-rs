//!
//! Common Library: service
//!

use bytemuck::NoUninit;
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread::JoinHandle;

use atomic::{Atomic, Ordering};
use crossbeam_channel as channel;
use my_logger::get_current_tid;
use pinky_swear::PinkySwear;

use commlib::{Clock, StopWatch, XmlReader};

use super::G_EXIT_CV;

const MAX_TASKS: usize = 4096_usize;
const RECV_WAIT_TIME: std::time::Duration = std::time::Duration::from_millis(60); // 60 ms

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

pub type ServiceFuncType = dyn FnOnce() + Send; // Note: tait object is always 'static, no need add 'static here

/// Service handle
pub struct ServiceHandle {
    id: u64,
    state: Atomic<NodeState>,

    tx: channel::Sender<Box<ServiceFuncType>>,
    rx: channel::Receiver<Box<ServiceFuncType>>,

    xml_config: RwLock<XmlReader>,

    //
    tid: Atomic<u64>,
    join_handle_opt: RwLock<Option<JoinHandle<()>>>,
}

impl ServiceHandle {
    ///
    pub fn new(id: u64, state: NodeState) -> Self {
        let (tx, rx) = channel::unbounded::<Box<ServiceFuncType>>();

        Self {
            id,
            state: Atomic::new(state),

            tx,
            rx,

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
    pub fn run_in_service(&self, cb: Box<dyn FnOnce() + Send>) {
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

    /// update
    fn update(&self);

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send>);

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool;

    /// 等待线程结束
    fn join(&self);
}

// 启动 service，service 需要使用 Arc 包装，否则无法跨线程 move.
// (采用独立函数是因为 self: &Arc<T> 和 泛型 会导致 ServiceRs 不是 object safe)
pub fn launch_service<T, I>(srv: &Arc<T>, initializer: I) -> bool
where
    T: ServiceRs + 'static,
    I: FnOnce() + Send + 'static,
{
    let join_handle_opt = start_service(&srv, srv.name(), initializer);
    proc_service_ready(srv.as_ref(), join_handle_opt)
}

// 开启 service 线程
fn start_service<T, I>(srv: &Arc<T>, name_of_thread: &str, initializer: I) -> Option<JoinHandle<()>>
where
    T: ServiceRs + 'static,
    I: FnOnce() + Send + 'static,
{
    //
    let handle = srv.get_handle();
    let tid = handle.tid();
    if tid > 0u64 {
        log::error!("service already started!!! tid={}", tid);
        return None;
    }

    //
    let (ready_prms, ready_pinky) = PinkySwear::<()>::new();

    //
    let srv2 = srv.clone();
    let exit_cv = Arc::clone(&G_EXIT_CV);

    let tname = name_of_thread.to_owned();
    let join_handle = std::thread::Builder::new()
        .name(tname.to_owned())
        .spawn(move || {
            let handle = srv2.get_handle();

            // update tid
            let tid = get_current_tid();
            handle.set_tid(tid);

            log::info!("srv({})[{}] spawn on thread: {}", handle.id(), tname, tid);

            // 服务线程初始化
            (initializer)();

            // 服务线程就绪通知( notify ready )
            ready_pinky.swear(());

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
    ready_prms.wait();
    Some(join_handle)
}

// 线程启动完成，保存 JoinHandle
fn proc_service_ready<T>(srv: &T, join_handle_opt: Option<JoinHandle<()>>) -> bool
where
    T: ServiceRs + 'static,
{
    let handle = srv.get_handle();

    if join_handle_opt.is_some() {
        // update join_handle
        {
            let mut join_handle_opt_mut = handle.join_handle_opt.write();
            (*join_handle_opt_mut) = join_handle_opt;
        }
        true
    } else {
        log::error!("[proc_service_ready] failed!!! tid: {}", handle.tid());
        false
    }
}

fn run_service<T>(srv: &T, service_name: &str)
where
    T: ServiceRs + 'static,
{
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
            // update thread local clock
            Clock::update();

            // update
            srv.update();

            // process async tasks
            let mut count = 0_usize;
            while count < MAX_TASKS {
                match handle.rx.recv_timeout(RECV_WAIT_TIME) {
                    Ok(cb) => {
                        cb();
                        count += 1;
                    }
                    Err(_err) => {
                        break;
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
            }
        }
    }
}
