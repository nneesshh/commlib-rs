use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs, G_SERVICE_DNS_RESOLVER};

use super::{ConnId, MessageIoNetwork};

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
#[repr(C)]
pub struct Connector {
    //
    srv_net: Arc<ServiceNetRs>,
    mi_network: Arc<MessageIoNetwork>,

    //
    pub name: String,
    pub ready_cb: Box<dyn Fn(Result<(ConnId, SocketAddr), String>) + Send + Sync>,
}

impl Connector {
    ///
    pub fn new<F>(
        mi_network: &Arc<MessageIoNetwork>,
        name: &str,
        ready_cb: F,
        srv_net: &Arc<ServiceNetRs>,
    ) -> Self
    where
        F: Fn(Result<(ConnId, SocketAddr), String>) + Send + Sync + 'static,
    {
        Self {
            srv_net: srv_net.clone(),
            mi_network: mi_network.clone(),

            name: name.to_owned(),
            ready_cb: Box::new(ready_cb),
        }
    }

    ///
    pub fn on_sock_addr_ready(self: &Arc<Self>, addr: SocketAddr) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // it is a regular SocketAddr, start connect directly
        let srv_net = self.srv_net.clone();
        let connector = self.clone();
        self.mi_network
            .connect_with_connector(&connector, addr, &srv_net);
    }

    ///
    pub fn start(self: &Arc<Self>, raddr: &str) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // try to parse as a regular SocketAddr first
        if let Ok(addr) = raddr.parse() {
            self.on_sock_addr_ready(addr);
            return;
        };

        // DNS resolve
        G_SERVICE_DNS_RESOLVER.resolve(self, raddr, &self.srv_net)
    }
}

///
pub fn insert_connector(srv_net: &ServiceNetRs, hd: ConnId, connector: &Arc<Connector>) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_CONNECTOR_STORAGE, g, {
        log::info!("[hd={}]({}) add connector", hd, connector.name);
        g.connector_table.insert(hd, connector.clone());
    });
}

///
pub fn remove_connector(srv_net: &ServiceNetRs, hd: ConnId) -> Option<Arc<Connector>> {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_CONNECTOR_STORAGE, g, {
        log::info!("[hd={}] remove connector", hd);
        g.connector_table.remove(&hd)
    })
}

///
pub fn on_connect_success(srv_net: &ServiceNetRs, hd: ConnId, sock_addr: SocketAddr) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_CONNECTOR_STORAGE, g, {
        //
        let connector_opt = g.connector_table.get(&hd);
        if let Some(connector) = connector_opt {
            (connector.ready_cb)(Ok((hd, sock_addr)));
        } else {
            log::error!("[hd={}] connector not found!!!", hd);
        }
    });
}

///
pub fn on_connect_failed(srv_net: &ServiceNetRs, hd: ConnId) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_CONNECTOR_STORAGE, g, {
        //
        let connector_opt = g.connector_table.get(&hd);
        if let Some(connector) = connector_opt {
            (connector.ready_cb)(Err("HandshakeError".to_owned()));
        } else {
            log::error!("[hd={}] connector not found!!!", hd);
        }
    });
}
