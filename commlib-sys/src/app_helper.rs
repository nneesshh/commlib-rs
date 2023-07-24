use crate::commlib_service::ServiceRs;

/// App: 应用框架
pub struct App {
    mtx: std::sync::Mutex<()>,
    services: Vec<crate::ServiceWrapper>,
    creators: hashbrown::HashMap<u64, Box<dyn FnOnce()->crate::ServiceWrapper>>,
}

impl App {
    /// Constructor
    pub fn new() -> Self {
        Self {
            mtx: std::sync::Mutex::default(),
            services: Vec::default(),
            creators: hashbrown::HashMap::default(),
        }
    }

    /// 注册 service creator
    pub fn register(&mut self, id: u64, creator: Box<dyn FnOnce()->crate::ServiceWrapper>)
    {
        self.creators.insert(id, creator).unwrap();
    }

    fn add_service(&mut self, srv: std::sync::Arc<dyn ServiceRs>) -> bool {
        self.mtx.lock().unwrap();
        for w in &self.services {
            let id = srv.get_handle().id();
            if (w.srv.get_handle().id() == id)
            {
                log::error!("App::add_service() failed!!! ID={}", id);
            }
            return false;
        }

        self.services.push(crate::ServiceWrapper { srv });
        true
    }

    /// 初始化 App
    pub fn init(&mut self) {

        crate::globals::G_SRV_SIGNAL.write().unwrap().init();
    }

    /// 启动 App
    pub fn start(&mut self) {
        crate::globals::G_SRV_SIGNAL.write().unwrap().start();
    }
}
