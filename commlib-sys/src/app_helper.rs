use crate::commlib_service::ServiceRs;
use std::sync::{Arc, Mutex, RwLock};

/// App: 应用框架RwLock<
pub struct App {

    services: Vec<crate::ServiceWrapper>,
    // Entry service: the last attached service
    pub entry: Option<Arc<RwLock<dyn ServiceRs>>>
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
        srv: Arc<RwLock<dyn ServiceRs>>,
    ) -> Option<Arc<RwLock<dyn ServiceRs>>> {
        for w in &*services {
            let id = srv.read().unwrap().get_handle().id();
            if w.srv.read().unwrap().get_handle().id() == id {
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
        let srv = Arc::new(RwLock::new(s));
        srv.write().unwrap().conf();
        srv.write().unwrap().start();

        // Update entry service
        self.entry = Self::add_service::<S>(&mut self.services, srv);
    }

    /// 启动 App
    pub fn start(&mut self) {
        // 配置 servie
        for w in &mut self.services {
            w.srv.write().unwrap().conf();
        }

        // 启动 servie
        for w in &mut self.services {
            w.srv.write().unwrap().start();
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
                let mut srv = w.srv.read().unwrap();

                log::info!(
                    "App:run() wait close .. ID={} state={:?}",
                    srv.get_handle().id(),
                    srv.get_handle().state()
                );
                if crate::State::Closed as u32 != srv.get_handle().state() as u32 {
                    exitflag = false;
                    break;
                }
            }

            if exitflag {
                for w in &self.services {
                    let mut srv = w.srv.write().unwrap();
                    srv.join()
                }
                break;
            }
        }
    }
}
