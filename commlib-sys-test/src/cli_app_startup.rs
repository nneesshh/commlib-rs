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

use commlib_sys::{connect_to_tcp_server, G_SERVICE_NET, G_SERVICE_SIGNAL};
use commlib_sys::{ConnId, NetPacketGuard, NodeState, ServiceRs};

use app_helper::Startup;

use crate::cli_conf::G_CLI_CONF;
use crate::cli_manager::G_MAIN;

use super::cli_service::CliService;
use super::cli_service::G_CLI_SERVICE;

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
pub fn exec(srv: &Arc<CliService>) {
    //
    cli_service_init(srv);

    //
    let srv2 = srv.clone();
    G_APP_STARTUP.with(|g| {
        let mut startup = g.borrow_mut();

        //
        startup.add_step("connect", move || do_connect_to_test_server(&srv2));

        // run
        startup.exec();
    });
}

/// Init in-service
fn cli_service_init(srv: &Arc<CliService>) -> bool {
    let handle = srv.get_handle();

    // ctrl-c stop, DEBUG ONLY
    G_SERVICE_SIGNAL.listen_sig_int(G_CLI_SERVICE.as_ref(), || {
        println!("WTF!!!!");
    });
    log::info!("\nGAME init ...\n");

    //
    app_helper::with_conf_mut!(G_CLI_CONF, cfg, { cfg.init(handle.xml_config()) });

    //
    handle.set_state(NodeState::Start);
    true
}

///
pub fn do_connect_to_test_server(srv: &Arc<CliService>) -> bool {
    //
    let raddr = app_helper::with_conf!(G_CLI_CONF, cfg, {
        std::format!("{}:{}", cfg.remote.addr, cfg.remote.port)
    });

    let conn_fn = |hd: ConnId| {
        log::info!("[hd={}] conn_fn", hd);

        hd.send(&G_SERVICE_NET, "hello, rust conn_fn".as_bytes());
    };

    let pkt_fn = |hd: ConnId, pkt: NetPacketGuard| {
        log::info!("[hd={}] msg_fn", hd);

        G_MAIN.with(|g| {
            let mut main_manager = g.borrow_mut();
            main_manager.proxy.on_net_packet(hd, pkt);
        });
    };

    let close_fn = |hd: ConnId| {
        log::info!("[hd={}] close_fn", hd);

        hd.send(&G_SERVICE_NET, "bye, rust close_fn".as_bytes());
    };

    //
    let hd_opt = connect_to_tcp_server(
        srv,
        "cli",
        raddr.as_str(),
        conn_fn,
        pkt_fn,
        close_fn,
        &G_SERVICE_NET,
    );

    //
    hd_opt.is_some()
}
