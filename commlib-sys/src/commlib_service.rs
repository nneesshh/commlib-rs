//!
//! Common Library: service
//!

use crossbeam::channel;

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

pub type ServiceFuncType = dyn FnOnce() + Send + Sync + 'static;

/// Service handle
pub struct ServiceHandle {
    pub id: u64,
    pub state: State,

    pub(crate) tx: std::sync::Arc<channel::Sender<Box<ServiceFuncType>>>,
    pub(crate) rx: std::sync::Arc<channel::Receiver<Box<ServiceFuncType>>>,

    pub(crate) tid: Option<std::thread::ThreadId>,

    pub(crate) clock: crate::Clock,
    pub(crate) xml_config: crate::XmlReader,
}

impl ServiceHandle {
    ///
    pub fn new(id: u64, state: State) -> ServiceHandle {
        let (tx, rx) = channel::unbounded::<Box<ServiceFuncType>>();

        Self {
            id,
            state,
            tx: std::sync::Arc::new(tx),
            rx: std::sync::Arc::new(rx),
            tid: None,
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
}

/// Service start a new single thread, and run callback in it.
pub trait ServiceRs {
    /// 获取 service 句柄
    fn get_handle(&self)->&ServiceHandle;

    /// 初始化 service
    fn init(&mut self);

    /// 启动 service 线程
    fn start(&mut self);

    /// 在 service 线程中执行回调任务
    fn run_in_service(&mut self, cb: Box<dyn FnOnce() + Send + Sync>);

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool;
}
