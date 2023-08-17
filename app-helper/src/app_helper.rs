use commlib_sys::*;

use crate::{with_conf_mut, Startup};

/// App: 应用框架RwLock<
pub struct App {
    app_name: String,
    services: Vec<ServiceWrapper>,
}

impl App {
    /// Constructor
    pub fn new(arg_vec: &Vec<std::ffi::OsString>, app_name: &str) -> Self {
        let mut app = Self {
            app_name: app_name.to_owned(),
            services: Vec::default(),
        };
        app.config(arg_vec, app_name);

        // attach default services
        app.attach(|| G_SERVICE_SIGNAL.as_ref());
        app.attach(|| G_SERVICE_NET.as_ref());

        app
    }

    /// App init
    pub fn init<C, I>(&mut self, mut creator: C, mut initializer: I)
    where
        C: FnOnce() -> &'static dyn ServiceRs,
        I: FnOnce(&'static dyn ServiceRs),
    {
        log::info!("App({}) startup ...", self.app_name);
        let srv = self.attach(creator);
        initializer(srv);
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
                if NodeState::Closed as u32 != w_srv_handle.state() as u32 {
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

    fn config(&mut self, arg_vec: &Vec<std::ffi::OsString>, srv_name: &str) {
        // init G_CONF
        with_conf_mut!(crate::G_CONF, cfg_mut, {
            cfg_mut.init(&arg_vec, srv_name);
        });

        // init logger
        let log_path = std::path::PathBuf::from("auto-legend");
        init_logger(&log_path, "testlog", spdlog::Level::Info as u32, true);
    }

    fn add_service(services: &mut Vec<ServiceWrapper>, srv: &'static dyn ServiceRs) {
        //
        let id = srv.get_handle().read().id();

        // 是否已经存在相同 id 的 service ?
        for w in &*services {
            let w_srv_handle = w.srv.get_handle().read();
            if w_srv_handle.id() == id {
                log::error!("App::add_service() failed!!! ID={}", id);
                return;
            }
        }

        //
        services.push(ServiceWrapper { srv });
        log::info!("App::add_service() ok, ID={}", id);
    }

    fn attach<F>(&mut self, mut creator: F) -> &'static dyn ServiceRs
    where
        F: FnOnce() -> &'static dyn ServiceRs,
    {
        let srv = creator();

        // attach xml node to custom service
        crate::with_conf!(crate::G_CONF, cfg, {
            let node_id = cfg.node_id;
            if let Some(xml_node) = cfg.get_xml_node(node_id) {
                let srv_type = xml_node.get_u64(vec!["srv"], 0);

                // xml config
                {
                    let mut handle_mut = srv.get_handle().write();
                    (*handle_mut).set_xml_config(xml_node.clone());
                }
            } else {
                log::error!("node {} xml config not found!!!", node_id);
            }
        });

        //
        srv.conf();
        let ready_pair = start_service(srv, srv.name());
        wait_service_ready(srv, ready_pair);

        // add server to app
        Self::add_service(&mut self.services, srv);

        //
        srv
    }
}
