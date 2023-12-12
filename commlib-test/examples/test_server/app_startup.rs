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

use net_packet::NetPacketGuard;

use commlib::with_tls;
use commlib::G_SERVICE_NET;
use commlib::{connect_to_redis, redis, tcp_server_listen};
use commlib::{ConnId, ServiceRs, TcpConn};

use app_helper::G_CONF;
use app_helper::{conf::Conf, Startup};

use crate::test_conf::G_TEST_CONF;
use crate::test_manager::G_MAIN;
use crate::test_service::G_TEST_SERVICE;

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
pub fn launch(_conf: &Arc<Conf>) {
    //
    let srv: &Arc<TestService> = &G_TEST_SERVICE;

    // pre-startup, main manager init
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
    //
    let g_conf = G_CONF.load();
    let connection_limit: usize = (g_conf.limit_players as f32 * 1.1_f32) as usize; // 0=no limit
    log::info!(
        "startup_network_listen: connection_limit={}",
        connection_limit
    );

    let conn_fn = |conn: Arc<TcpConn>| {
        let hd = conn.hd;
        log::info!("[hd={}] conn_fn", hd);

        //
        G_MAIN.with(|g| {
            let mut main_manager = g.borrow_mut();

            let push_encrypt_token = true; // 是否推送 encrypt token
            main_manager
                .c2s_proxy
                .on_incomming_conn(&conn, push_encrypt_token);
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
        let addr = std::format!("{}:{}", cfg.my.addr.as_str(), cfg.my.port);
        tcp_server_listen(
            srv,
            addr.as_str(),
            conn_fn,
            pkt_fn,
            close_fn,
            connection_limit,
            &G_SERVICE_NET,
        );
    });

    //
    true
}
