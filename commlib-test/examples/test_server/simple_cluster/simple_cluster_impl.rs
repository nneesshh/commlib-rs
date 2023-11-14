use commlib::{NetProxy, PacketType, ServiceRs, TcpConn};
use net_packet::CmdId;

use std::io::{self};
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;

use super::discovery_server::DiscoveryServer;
use super::participant::Participant;

///
pub enum SimpleClusterPeerType {
    DiscoveryServer,
    Participant,
}

///
pub struct SimpleCluster {
    pub r#type: SimpleClusterPeerType,
    pub name: String, // peer name

    pub s2s_proxy: NetProxy, // server to server

    //
    server_opt: Option<Arc<DiscoveryServer>>,

    participant_opt: Option<Arc<Participant>>,
}

impl SimpleCluster {
    ///
    pub fn new_discover_server(name: &str) -> Self {
        Self {
            r#type: SimpleClusterPeerType::DiscoveryServer,
            name: name.to_owned(),

            s2s_proxy: NetProxy::new(PacketType::Server),

            server_opt: None,

            participant_opt: None,
        }
    }

    ///
    pub fn new_participant(name: &str, raddr: &str) -> Self {
        Self {
            r#type: SimpleClusterPeerType::Participant,
            name: name.to_owned(),

            s2s_proxy: NetProxy::new(PacketType::Server),

            server_opt: None,

            participant_opt: None,
        }
    }
}

///
pub fn startup_network_listen(srv: &Arc<dyn ServiceRs>) -> bool {
    //
    let connection_limit: usize = 0; // 0=no limit
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
