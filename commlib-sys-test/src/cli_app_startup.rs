//!
//! G_CLI_APP_STARTUP
//!
//! example for resume:
//! '''
//!     G_CLI_APP_STARTUP.with(|g| {
//!         let mut startup = g.borrow_mut();
//!         startup.resume();
//!     });
//! '''

use parking_lot::RwLock;
use std::sync::Arc;

use commlib_sys::service_net::{ConnId, NetPacketGuard};
use commlib_sys::{connect_to_tcp_server, create_tcp_client};
use commlib_sys::{NodeState, ServiceHandle, ServiceRs, TcpCallbacks};
use commlib_sys::{G_SERVICE_NET, G_SERVICE_SIGNAL};

use app_helper::Startup;

use super::cli_service::CliService;
use super::cli_service::G_CLI_SERVICE;

use crate::cli_conf::G_CLI_CONF;
use crate::cli_manager::G_TEST_MANAGER;

thread_local! {
    ///
    pub static G_CLI_APP_STARTUP: std::cell::RefCell<Startup> = {
        std::cell::RefCell::new(Startup::new("app"))
    };
}

///
pub fn resume(srv: &Arc<CliService>) {
    srv.run_in_service(Box::new(|| {
        //
        G_CLI_APP_STARTUP.with(|g| {
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
    G_CLI_APP_STARTUP.with(|g| {
        let mut startup = g.borrow_mut();

        //
        startup.add_step("connect", move || do_connect_to_test_server(&srv2));

        // run
        startup.exec();
    });
}

/// Init in-service
fn cli_service_init(srv: &Arc<CliService>) -> bool {
    let mut handle_mut = srv.get_handle().write();

    // ctrl-c stop, DEBUG ONLY
    G_SERVICE_SIGNAL.listen_sig_int(G_CLI_SERVICE.as_ref(), || {
        println!("WTF!!!!");
    });
    log::info!("\nGAME init ...\n");

    //
    app_helper::with_conf_mut!(G_CLI_CONF, cfg, { cfg.init(handle_mut.xml_config()) });

    //
    handle_mut.set_state(NodeState::Start);
    true
}

///
pub fn do_connect_to_test_server(srv: &Arc<CliService>) -> bool {
    //
    let mut raddr = app_helper::with_conf!(G_CLI_CONF, cfg, {
        std::format!("{}:{}", cfg.remote.addr, cfg.remote.port)
    });

    let cli = create_tcp_client(srv, "cli", raddr.as_str());

    let conn_fn = |hd: ConnId| {
        log::info!("[hd={:?}] conn_fn", hd);

        hd.send(&G_SERVICE_NET, "hello, rust conn_fn".as_bytes());
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

        hd.send(&G_SERVICE_NET, "bye, rust stopped_cb".as_bytes());
    };

    //
    connect_to_tcp_server(
        srv,
        "",
        raddr.as_str(),
        conn_fn,
        pkt_fn,
        stopped_cb,
        &G_SERVICE_NET,
    );

    //
    true
}
