use commlib_sys::*;

/// App: 应用框架RwLock<
pub struct App {
    services: Vec<ServiceWrapper>,
}

impl App {
    /// Constructor
    pub fn new(arg_vec: &Vec<std::ffi::OsString>, srv_name: &str) -> Self {
        let mut app = Self {
            services: Vec::default(),
        };
        app.init(arg_vec, srv_name);
        app.start();
        app
    }

    fn init(&mut self, arg_vec: &Vec<std::ffi::OsString>, srv_name: &str) {
        // init G_CONF
        {
            let mut g_conf_mut = crate::G_CONF.write();
            (*g_conf_mut).init(&arg_vec, srv_name);
        }

        // init logger
        let log_path = std::path::PathBuf::from("auto-legend");
        init_logger(&log_path, "testlog", spdlog::Level::Info as u32, true);

        Self::add_service(&mut self.services, G_SERVICE_SIGNAL.as_ref());
        Self::add_service(&mut self.services, G_SERVICE_NET.as_ref());
    }

    fn start(&mut self) {
        // 配置 servie
        for w in &mut self.services {
            w.srv.conf();
        }

        // 启动 servie
        for w in &mut self.services {
            start_service(w.srv, w.srv.name());
        }
    }

    fn add_service(services: &mut Vec<ServiceWrapper>, srv: &'static dyn ServiceRs) {
        // 是否已经存在相同 id 的 service ?
        for w in &*services {
            let id = srv.get_handle().read().id();
            let w_srv_handle = w.srv.get_handle().read();
            if w_srv_handle.id() == id {
                log::error!("App::add_service() failed!!! ID={}", id);
                return;
            }
        }
        services.push(ServiceWrapper { srv });
    }

    /// 添加 service
    pub fn attach<C>(&mut self, mut creator: C)
    where
        C: FnMut() -> &'static dyn ServiceRs,
    {
        let srv = creator();
        srv.conf();
        start_service(srv, srv.name());

        // add server to app
        Self::add_service(&mut self.services, srv);
    }

    /// App  等待直至服务关闭
    pub fn run(self) {
        let cv = G_EXIT_CV.clone();
        let &(ref lock, ref cvar) = &*cv;
        loop {
            // wait quit signal
            let mut quit = lock.lock();
            cvar.wait(&mut quit);

            let mut exitflag = true;
            for w in &self.services {
                let w_srv_handle = w.srv.get_handle().read();
                log::info!(
                    "App:run() wait close .. ID={} state={:?}",
                    w_srv_handle.id(),
                    w_srv_handle.state()
                );
                if State::Closed as u32 != w_srv_handle.state() as u32 {
                    exitflag = false;
                    break;
                }
            }

            if exitflag {
                for w in &self.services {
                    w.srv.join();
                }
                break;
            }
        }
    }
}
