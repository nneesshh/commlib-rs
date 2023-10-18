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
use commlib::{connect_to_tcp_server, G_SERVICE_NET};
use commlib::{ConnId, NetPacketGuard, ServiceRs, TcpConn};

use app_helper::{conf::Conf, Startup};

use super::cli_service::CliService;
use crate::cli_conf::G_CLI_CONF;
use crate::cli_manager::G_MAIN;

thread_local! {
    ///
    pub static G_APP_STARTUP: std::cell::RefCell<Startup> = {
        std::cell::RefCell::new(Startup::new("app"))
    };
}

///
pub fn resume(srv: &Arc<CliService>) {
    srv.run_in_service(Box::new(|| {
        //
        G_APP_STARTUP.with(|g| {
            let mut startup = g.borrow_mut();
            startup.resume();
        });
    }));
}

///
pub fn launch(srv: &Arc<CliService>, conf: &Arc<Conf>) {
    //test_http_client(srv);

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
        startup.add_step("robots connect", move || {
            //
            const ROBOT_NUM: usize = 2;
            for i in 0..ROBOT_NUM {
                do_connect_to_test_server(&step_srv, i + 1);
            }
            true
        });

        // run startup
        startup.exec();
    });

    // startup over, main manager lazy init
    G_MAIN.with(|g| {
        let mut main_manager = g.borrow_mut();
        main_manager.lazy_init(srv);
    });
}

///
pub fn do_connect_to_test_server(srv: &Arc<CliService>, index: usize) -> bool {
    //
    let raddr = with_tls!(G_CLI_CONF, cfg, {
        std::format!("{}:{}", cfg.remote.addr, cfg.remote.port)
    });

    let conn_fn = |conn: Arc<TcpConn>| {
        let hd = conn.hd;
        log::info!("[hd={}] conn_fn", hd);

        //
        G_MAIN.with(|g| {
            let mut cli_manager = g.borrow_mut();

            let push_encrypt_token = false;
            cli_manager
                .proxy
                .on_incomming_conn(conn.as_ref(), push_encrypt_token);
        });
    };

    let pkt_fn = |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
        let hd = conn.hd;
        log::info!("[hd={}] msg_fn", hd);

        G_MAIN.with(|g| {
            let mut main_manager = g.borrow_mut();
            main_manager.proxy.on_net_packet(conn.as_ref(), pkt);
        });
    };

    let close_fn = |hd: ConnId| {
        log::info!("[hd={}] close_fn", hd);

        G_MAIN.with(|g| {
            let mut main_manager = g.borrow_mut();
            main_manager.proxy.on_hd_lost(hd);
        });
    };

    //
    let cli_opt = connect_to_tcp_server(
        &srv,
        std::format!("cli{}", index).as_str(),
        raddr.as_str(),
        conn_fn,
        pkt_fn,
        close_fn,
        &G_SERVICE_NET,
    );

    //
    cli_opt.is_some()
}

use serde_json::json;

use crate::robot::G_ROBOT_MANAGER;
use commlib::G_SERVICE_HTTP_CLIENT;

fn test_http_client(srv: &Arc<CliService>) {
    let body =
        json!({"foo": false, "bar": null, "answer": 42, "list": [null, "world", true]}).to_string();

    //
    let srv2 = srv.clone();
    G_SERVICE_HTTP_CLIENT.http_post(
        "http://127.0.0.1:7878",
        vec!["Content-Type: application/json".to_owned()],
        body,
        move |code, resp| {
            //
            srv2.run_in_service(Box::new(move || {
                log::info!("hello http code: {}, resp: {}", code, resp);
            }));
        },
    )
}
