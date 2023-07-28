use crate::{commlib_service::ServiceRs, start_service};
use std::sync::{Arc, Mutex, RwLock};

/// App: 应用框架RwLock<
pub struct App {

    services: Vec<crate::ServiceWrapper>,
    // Entry service: the last attached service
    pub entry: Option<Arc<dyn ServiceRs>>
}

impl App {
    /// Constructor
    pub fn new() -> Self {
        let mut app = Self {
            services: Vec::default(),
            entry: None,
        };
        app.init();
        app
    }

    fn init(&mut self) {
        Self::add_service::<crate::ServiceSignalRs>(
            &mut self.services,
            crate::globals::G_SERVICE_SIGNAL.clone(),
        );
        Self::add_service::<crate::ServiceNetRs>(
            &mut self.services,
            crate::globals::G_SERVICE_NET.clone(),
        );
    }

    fn add_service<S>(
        services: &mut Vec<crate::ServiceWrapper>,
        srv: Arc<dyn ServiceRs>,
    ) -> Option<Arc<dyn ServiceRs>> {
        // 是否已经存在相同 id 的 service ?
        for w in &*services {
            let id = srv.get_handle().read().unwrap().id();
            let w_srv_handle = w.srv.get_handle().read().unwrap();
            if w_srv_handle.id() == id {
                log::error!("App::add_service() failed!!! ID={}", id);
                return None;
            }
        }

        let ret = Some(srv.clone());
        services.push(crate::ServiceWrapper { srv });
        ret
    }

    /// 添加 service
    pub fn attach<S, C>(&mut self, mut creator: C)
    where
        S: ServiceRs + 'static,
        C: FnMut() -> S,
    {
        let s = creator();
        let srv = Arc::new(s);
        srv.conf();
        //start_service(w.srv, "");

        // Update entry service
        self.entry = Self::add_service::<S>(&mut self.services, srv);
    }

    /// 启动 App
    pub fn start(&mut self) {
        // 配置 servie
        for w in &mut self.services {
            w.srv.conf();
        }

        // 启动 servie
        for w in &mut self.services {
            //start_service(w.srv, "");
        }
    }

    /// App  等待直至服务关闭
    pub fn run(self) {
        let cv = crate::G_EXIT_CV.clone();
        let &(ref lock, ref cvar) = &*cv;
        loop {
            // wait quit signal
            let mut quit = lock.lock().unwrap();
            quit = cvar.wait(quit).unwrap();

            let mut exitflag = true;
            for w in &self.services {
                let w_srv_handle = w.srv.get_handle().read().unwrap();
                log::info!(
                    "App:run() wait close .. ID={} state={:?}",
                    w_srv_handle.id(),
                    w_srv_handle.state()
                );
                if crate::State::Closed as u32 != w_srv_handle.state() as u32 {
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
