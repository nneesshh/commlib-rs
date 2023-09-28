//!
//! G_APP_STARTUP
//!
//! example for resume:
//! '''
//!     G_APP_STARTUP.with(|g| {
//!         let mut startup = g.borrow_mut();
//!         startup.resume();
//!     });
//! '''

use std::sync::Arc;

use commlib::with_tls;
use commlib::G_SERVICE_NET;
use commlib::{connect_to_redis, listen_tcp_addr, redis};
use commlib::{ConnId, NetPacketGuard, ServiceRs, TcpConn};

use app_helper::{conf::Conf, Startup};

use crate::test_conf::G_TEST_CONF;
use crate::test_manager::G_MAIN;

use super::test_service::TestService;

thread_local! {
    ///
    pub static G_APP_STARTUP: std::cell::RefCell<Startup> = {
        std::cell::RefCell::new(Startup::new("app"))
    };
}

///
pub fn resume(srv: &Arc<TestService>) {
    srv.run_in_service(Box::new(|| {
        //
        G_APP_STARTUP.with(|g| {
            let mut startup = g.borrow_mut();
            startup.resume();
        });
    }));
}

///
pub fn launch(srv: &Arc<TestService>, conf: &Arc<Conf>) {
    // pre-startup, main manager init
    commlib::ossl_init();

    G_MAIN.with(|g| {
        let mut main_manager = g.borrow_mut();
        main_manager.init(srv);
    });

    // startup step by step
    G_APP_STARTUP.with(|g| {
        let mut startup = g.borrow_mut();

        // step:
        let step_srv = srv.clone();
        startup.add_step("start network listen", move || {
            //
            startup_network_listen(&step_srv)
        });

        // run startup
        startup.exec();
    });

    // startup over, main manager lazy init
    G_MAIN.with(|g| {
        let mut main_manager = g.borrow_mut();
        main_manager.lazy_init();
    });
}

///
pub fn startup_network_listen(srv: &Arc<TestService>) -> bool {
    // TODO: let thread_num: u32 = 1;
    // TODO: let connection_limit: u32 = 0; // 0=no limit

    let conn_fn = |conn: Arc<TcpConn>| {
        let hd = conn.hd;
        log::info!("[hd={}] conn_fn", hd);

        //
        G_MAIN.with(|g| {
            let mut main_manager = g.borrow_mut();

            let push_encrypt_token = true; // 是否推送 encrypt token
            main_manager
                .c2s_proxy
                .on_incomming_conn(conn.as_ref(), push_encrypt_token);
        });
    };

    let pkt_fn = |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
        let hd = conn.hd;
        log::info!("[hd={}] msg_fn", hd);

        G_MAIN.with(|g| {
            let mut main_manager = g.borrow_mut();
            main_manager.c2s_proxy.on_net_packet(conn.as_ref(), pkt);
        });
    };

    let close_fn = |hd: ConnId| {
        log::info!("[hd={}] close_fn", hd);

        G_MAIN.with(|g| {
            let mut main_manager = g.borrow_mut();
            main_manager.c2s_proxy.on_hd_lost(hd);
        });
    };

    //
    with_tls!(G_TEST_CONF, cfg, {
        let listener_id = listen_tcp_addr(
            srv,
            cfg.my.addr.clone(),
            cfg.my.port,
            conn_fn,
            pkt_fn,
            close_fn,
            &G_SERVICE_NET,
        );
        log::info!("listener {} ready.", listener_id);
    });

    //
    true
}
