//!
//! App Startup
//!
//! example for resume:
//! '''
//!     G_APP_STARTUP.with(|g| {
//!         let mut startup = g.borrow_mut();
//!         startup.resume();
//!     });
//! '''

use commlib_sys::G_SERVICE_NET;
use commlib_sys::{ServerCallbacks, ServiceRs};

use app_helper::Startup;

use crate::test_conf::G_TEST_CONF;
use crate::test_manager::G_TEST_MANAGER;

thread_local! {
    ///
    pub static G_APP_STARTUP: std::cell::RefCell<Startup> = {
        std::cell::RefCell::new(Startup::new("app"))
    };
}

///
pub fn resume(srv: &'static dyn ServiceRs) {
    srv.run_in_service(Box::new(|| {
        //
        G_APP_STARTUP.with(|g| {
            let mut startup = g.borrow_mut();
            startup.resume();
        });
    }));
}

///
pub fn exec(srv: &'static dyn ServiceRs) {
    srv.run_in_service(Box::new(|| {
        //
        G_APP_STARTUP.with(|g| {
            let mut startup = g.borrow_mut();

            //
            startup.add_step("start network listen", || startup_network_listen(srv));

            // run
            startup.exec();
        });
    }));
}

///
pub fn startup_network_listen(srv: &'static dyn ServiceRs) -> bool {
    // TODO: let thread_num: u32 = 1;
    // TODO: let connection_limit: u32 = 0; // 0=no limit

    let mut callbacks = ServerCallbacks::new();
    callbacks.srv = Some(srv);
    callbacks.conn_fn = Box::new(|hd| {
        log::info!("[hd={:?}] conn_fn", hd);

        hd.send(G_SERVICE_NET.as_ref(), "hello, rust conn_fn".as_bytes());
    });

    callbacks.msg_fn = Box::new(|hd, pkt| {
        log::info!("[hd={:?}] msg_fn", hd);

        G_TEST_MANAGER.with(|g| {
            let mut test_manager = g.borrow_mut();
            test_manager.server_proxy.on_net_packet(hd, pkt);
        });
    });

    callbacks.stopped_cb = Box::new(|hd| {
        log::info!("[hd={:?}] stopped_cb", hd);

        hd.send(G_SERVICE_NET.as_ref(), "bye, rust stopped_cb".as_bytes());
    });

    app_helper::with_conf!(G_TEST_CONF, cfg, {
        G_SERVICE_NET.listen(cfg.my.addr.clone(), cfg.my.port, callbacks, srv);
    });

    //
    true
}
