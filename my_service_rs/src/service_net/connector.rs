use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;

use commlib::with_tls_mut;

use super::dns_resolve;
use crate::{ServiceNetRs, ServiceRs};

use super::low_level_network::MessageIoNetwork;
use super::ConnId;

thread_local! {
    static G_CONNECTOR_STORAGE: UnsafeCell<ConnectorStorage> = UnsafeCell::new(ConnectorStorage::new());
}

struct ConnectorStorage {
    /// connector table
    connector_table: hashbrown::HashMap<ConnId, Arc<Connector>>,
}

impl ConnectorStorage {
    ///
    pub fn new() -> Self {
        Self {
            connector_table: hashbrown::HashMap::new(),
        }
    }
}

/// Connector
pub struct Connector {
    //
    pub name: String,
    pub connect_fn: Box<dyn Fn(Result<(ConnId, SocketAddr), String>) + Send + Sync>,

    //
    netctrl: Arc<MessageIoNetwork>,
    srv_net: Arc<ServiceNetRs>,
}

impl Connector {
    ///
    pub fn new<F>(
        name: &str,
        connect_fn: F,
        netctrl: &Arc<MessageIoNetwork>,
        srv_net: &Arc<ServiceNetRs>,
    ) -> Self
    where
        F: Fn(Result<(ConnId, SocketAddr), String>) + Send + Sync + 'static,
    {
        Self {
            name: name.to_owned(),
            connect_fn: Box::new(connect_fn),

            netctrl: netctrl.clone(),
            srv_net: srv_net.clone(),
        }
    }

    ///
    pub fn on_addr_ready(self: &Arc<Self>, addr: SocketAddr) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // it is a regular SocketAddr, start connect directly
        let srv_net = self.srv_net.clone();
        let connector = self.clone();
        self.netctrl
            .connect_with_connector(&connector, addr, &srv_net);
    }

    ///
    pub fn start(self: &Arc<Self>, raddr: &str) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // try to parse as a regular SocketAddr first
        if let Ok(addr) = raddr.parse() {
            self.on_addr_ready(addr);
            return;
        };

        // DNS resolve
        dns_resolve(self, raddr, &self.srv_net)
    }
}

///
pub fn on_connector_connect_ok(srv_net: &ServiceNetRs, hd: ConnId, sock_addr: SocketAddr) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_CONNECTOR_STORAGE, g, {
        //
        let connector_opt = g.connector_table.get(&hd);
        if let Some(connector) = connector_opt {
            // success
            (connector.connect_fn)(Ok((hd, sock_addr)));

            //
            remove_connector(g, hd);
        } else {
            log::error!("[hd={}] connector not found!!!", hd);
        }
    });
}

///
pub fn on_connector_connect_err(srv_net: &ServiceNetRs, hd: ConnId) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_CONNECTOR_STORAGE, g, {
        //
        let connector_opt = g.connector_table.get(&hd);
        if let Some(connector) = connector_opt {
            // failed
            (connector.connect_fn)(Err("HandshakeError".to_owned()));

            //
            remove_connector(g, hd);
        } else {
            log::error!("[hd={}] connector not found!!!", hd);
        }
    });
}

///
#[inline(always)]
pub fn insert_connector(srv_net: &ServiceNetRs, hd: ConnId, connector: &Arc<Connector>) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_CONNECTOR_STORAGE, g, {
        //log::info!("[hd={}]({}) add connector", hd, connector.name);
        g.connector_table.insert(hd, connector.clone());
    });
}

#[inline(always)]
fn remove_connector(storage: &mut ConnectorStorage, hd: ConnId) -> Option<Arc<Connector>> {
    if let Some(connetor) = storage.connector_table.remove(&hd) {
        //log::info!("[hd={}]({}) remove connector", hd, connetor.name);
        Some(connetor)
    } else {
        log::error!("[hd={}] connector not found!!!", hd);
        None
    }
}
