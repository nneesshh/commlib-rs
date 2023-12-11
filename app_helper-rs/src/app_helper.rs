use std::sync::Arc;

use commlib::{proc_service_ready, start_network, start_service};
use commlib::{NodeState, ServiceRs};
use commlib::{G_EXIT_CV, G_SERVICE_HTTP_CLIENT, G_SERVICE_NET, G_SERVICE_SIGNAL};

use crate::conf::Conf;
use crate::G_CONF;

/// App: 应用框架
pub struct App {
    app_name: String,
    services: Vec<Arc<dyn ServiceRs + 'static>>,
}

impl App {
    /// Constructor
    pub fn new(arg_vec: &Vec<std::ffi::OsString>, app_name: &str) -> Self {
        let mut app = Self {
            app_name: app_name.to_owned(),
            services: Vec::default(),
        };
        app.config(arg_vec, app_name);

        // attach default services -- signal
        app.attach(&G_SERVICE_SIGNAL, |_conf| {
            // do nothing
        });

        // attach default services -- net
        app.attach(&G_SERVICE_NET, |_conf| {
            // 启动 network
            start_network(&G_SERVICE_NET);
        });

        // attach default services -- http_client
        app.attach(&G_SERVICE_HTTP_CLIENT, |_conf| {
            // do nothing
        });

        app
    }

    /// App init
    pub fn init<T, F>(&mut self, srv: &Arc<T>, initializer: F)
    where
        T: ServiceRs + 'static,
        F: FnOnce(&Arc<Conf>) + Send + Sync + 'static,
    {
        log::info!("App({}) startup ...", self.app_name);
        self.attach(srv, initializer);
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
            for srv in &self.services {
                let srv_handle = srv.get_handle();
                log::info!(
                    "App:run() wait close .. App={} ID={} state={:?}",
                    self.app_name,
                    srv_handle.id(),
                    srv_handle.state()
                );
                if NodeState::Closed as u32 != srv_handle.state() as u32 {
                    exitflag = false;
                    break;
                }
            }

            if exitflag {
                for srv in &self.services {
                    srv.join();
                }
                break;
            }
        }
    }

    fn config(&mut self, arg_vec: &Vec<std::ffi::OsString>, srv_name: &str) {
        // init G_CONF
        let mut conf_mut = Conf::new();
        conf_mut.init(&arg_vec, srv_name);
        G_CONF.store(Arc::new(conf_mut));

        // init logger
        let log_path = std::path::PathBuf::from("auto-legend");
        my_logger::init(&log_path, "testlog", my_logger::LogLevel::Info as u16, true);
    }

    fn add_service<T>(services: &mut Vec<Arc<dyn ServiceRs>>, service: &Arc<T>)
    where
        T: ServiceRs + 'static,
    {
        //
        let id = service.get_handle().id();
        let name = service.name();

        // 是否已经存在相同 id 的 service ?
        for srv in &*services {
            let srv_handle = srv.get_handle();
            if srv_handle.id() == id {
                log::error!("App::add_service [{}] failed!!! ID={}!!!", name, id);
                return;
            }
        }

        //
        services.push(service.clone());
        log::info!("App::add_service [{}] ok, ID={}", name, id);
    }

    fn attach<T, F>(&mut self, srv: &Arc<T>, initializer: F)
    where
        T: ServiceRs + 'static,
        F: FnOnce(&Arc<Conf>) + Send + Sync + 'static,
    {
        // attach xml node to custom service
        let g_conf = G_CONF.load();
        let node_id = g_conf.node_id;
        if let Some(xml_node) = g_conf.get_xml_node(node_id) {
            //
            let srv_type = xml_node.get_u64(vec!["srv"], 0);
            log::info!(
                "srv({}): node {} srv_type {}",
                srv.get_handle().id(),
                node_id,
                srv_type
            );

            // set xml config
            srv.get_handle().set_xml_config(xml_node.clone());
        } else {
            log::error!("node {} xml config not found!!!", node_id);
        }

        //
        srv.conf();

        //
        let g_conf2 = g_conf.clone();
        let join_handle_opt = start_service(&srv, srv.name(), move || {
            initializer(&g_conf2);
        });
        proc_service_ready(srv.as_ref(), join_handle_opt);

        // add service (nudge the compiler to infer the correct type)
        Self::add_service(&mut self.services, &srv);
    }
}
