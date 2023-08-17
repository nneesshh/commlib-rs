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

use commlib_sys::service_net::{ConnId, NetPacketGuard};
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

    let conn_fn = |hd: ConnId| {
        log::info!("[hd={:?}] conn_fn", hd);

        hd.send(G_SERVICE_NET.as_ref(), "hello, rust conn_fn".as_bytes());
    };

    let pkt_fn = |hd: ConnId, pkt: NetPacketGuard| {
        log::info!("[hd={:?}] msg_fn", hd);

        G_TEST_MANAGER.with(|g| {
            let mut test_manager = g.borrow_mut();
            test_manager.server_proxy.on_net_packet(hd, pkt);
        });
    };

    let stopped_cb = |hd: ConnId| {
        log::info!("[hd={:?}] stopped_cb", hd);

        hd.send(G_SERVICE_NET.as_ref(), "bye, rust stopped_cb".as_bytes());
    };

    let callbacks = ServerCallbacks::new(srv, conn_fn, pkt_fn, stopped_cb);

    //
    app_helper::with_conf!(G_TEST_CONF, cfg, {
        G_SERVICE_NET.listen(cfg.my.addr.clone(), cfg.my.port, callbacks, srv);
    });

    //
    true
}
