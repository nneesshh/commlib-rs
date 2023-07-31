use crate::{init_logger, start_service, ServiceRs};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

/// App: 应用框架RwLock<
pub struct App {
    services: Vec<crate::ServiceWrapper>,
}

impl App {
    /// Constructor
    pub fn new() -> Self {
        let mut app = Self {
            services: Vec::default(),
        };
        app.init();
        app
    }

    fn init(&mut self) {
        //init_logger();

        Self::add_service(&mut self.services, crate::globals::G_SERVICE_SIGNAL.clone());
        Self::add_service(&mut self.services, crate::globals::G_SERVICE_NET.clone());
    }

    fn add_service(services: &mut Vec<crate::ServiceWrapper>, srv: Arc<dyn ServiceRs>) {
        // 是否已经存在相同 id 的 service ?
        for w in &*services {
            let id = srv.get_handle().read().id();
            let w_srv_handle = w.srv.get_handle().read();
            if w_srv_handle.id() == id {
                log::error!("App::add_service() failed!!! ID={}", id);
                return;
            }
        }
        services.push(crate::ServiceWrapper { srv });
    }

    /// 添加 service
    pub fn attach<C>(&mut self, mut creator: C)
    where
        C: FnMut() -> Arc<dyn ServiceRs>,
    {
        let srv = creator();
        srv.conf();
        start_service(&srv, "");

        // add server to app
        Self::add_service(&mut self.services, srv);
    }

    /// 启动 App
    pub fn start(&mut self) {
        // 配置 servie
        for w in &mut self.services {
            w.srv.conf();
        }

        // 启动 servie
        for w in &mut self.services {
            if (w.srv.is_cxx()) {
                w.srv.start_cxx_service();
            } else {
                start_service(&w.srv, w.srv.name());
            }
        }
    }

    /// App  等待直至服务关闭
    pub fn run(self) {
        let cv = crate::G_EXIT_CV.clone();
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
