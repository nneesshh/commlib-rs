use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;

use commlib::with_tls_mut;

use crate::{ServiceNetRs, ServiceRs};

use super::low_level_network::MessageIoNetwork;
use super::{ConnId, ListenerId};

thread_local! {
    static G_LISTENER_STORAGE: UnsafeCell<ListenerStorage> = UnsafeCell::new(ListenerStorage::new());
}

struct ListenerStorage {
    /// listener table
    listener_table: hashbrown::HashMap<ListenerId, Arc<Listener>>,
}

impl ListenerStorage {
    ///
    pub fn new() -> Self {
        Self {
            listener_table: hashbrown::HashMap::new(),
        }
    }
}

/// Listener
pub struct Listener {
    //
    pub name: String,
    pub listen_fn: Box<dyn Fn(Result<(ListenerId, SocketAddr), String>) + Send + Sync>,
    pub accept_fn: Box<dyn Fn(ListenerId, ConnId, SocketAddr) + Send + Sync>,

    //
    netctrl: Arc<MessageIoNetwork>,
    srv_net: Arc<ServiceNetRs>,
}

impl Listener {
    ///
    pub fn new<L, A>(
        name: &str,
        listen_fn: L,
        accept_fn: A,
        netctrl: &Arc<MessageIoNetwork>,
        srv_net: &Arc<ServiceNetRs>,
    ) -> Self
    where
        L: Fn(Result<(ListenerId, SocketAddr), String>) + Send + Sync + 'static,
        A: Fn(ListenerId, ConnId, SocketAddr) + Send + Sync + 'static,
    {
        Self {
            name: name.to_owned(),
            listen_fn: Box::new(listen_fn),
            accept_fn: Box::new(accept_fn),

            netctrl: netctrl.clone(),
            srv_net: srv_net.clone(),
        }
    }

    ///
    pub fn listen_with_tcp(self: &Arc<Self>, addr: &str) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        self.netctrl.listen_with_tcp(self, addr, &self.srv_net);
    }

    ///
    // pub fn listen_with_ssl(self: &Arc<Self>, addr: &str, cert_path: &str, pri_key_path: &str) {
    //     // 运行于 srv_net 线程
    //     assert!(self.srv_net.is_in_service_thread());

    //     let cert_path = std::path::PathBuf::from(cert_path);
    //     let pri_key_path = std::path::PathBuf::from(pri_key_path);
    //     self.netctrl
    //         .listen_with_ssl(self, addr, cert_path, pri_key_path, &self.srv_net);
    // }

    ///
    #[cfg(feature = "websocket")]
    pub fn listen_with_ws(self: &Arc<Self>, addr: &str) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        self.netctrl.listen_with_ws(self, addr, &self.srv_net);
    }
}

///
pub fn on_listener_listen(srv_net: &ServiceNetRs, listener_id: ListenerId, sock_addr: SocketAddr) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_LISTENER_STORAGE, g, {
        //
        let listener_opt = g.listener_table.get(&listener_id);
        if let Some(listener) = listener_opt {
            // success
            (listener.listen_fn)(Ok((listener_id, sock_addr)));
        } else {
            log::error!("[listener_id={}] listener not found!!!", listener_id);
        }
    });
}

///
pub fn on_listener_accept(
    srv_net: &ServiceNetRs,
    listener_id: ListenerId,
    hd: ConnId,
    sock_addr: SocketAddr,
) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_LISTENER_STORAGE, g, {
        //
        let listener_opt = g.listener_table.get(&listener_id);
        if let Some(listener) = listener_opt {
            // success
            (listener.accept_fn)(listener_id, hd, sock_addr);
        } else {
            log::error!(
                "[listener_id={}] listener not found!!! hd={}!!!",
                listener_id,
                hd
            );
        }
    });
}

///
#[inline(always)]
pub fn insert_listener(srv_net: &ServiceNetRs, listener_id: ListenerId, listener: &Arc<Listener>) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_LISTENER_STORAGE, g, {
        //log::info!("[listener_id={}]({}) add listener", listener_id, listener.name);
        g.listener_table.insert(listener_id, listener.clone());
    });
}

///
#[allow(dead_code)]
pub fn remove_listener(srv_net: &ServiceNetRs, listener_id: ListenerId) -> Option<Arc<Listener>> {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_LISTENER_STORAGE, g, {
        if let Some(listener) = g.listener_table.remove(&listener_id) {
            //log::info!("[listener_id={}]({}) remove listener", listener_id, listener.name);
            Some(listener)
        } else {
            log::error!("[listener_id={}] listener not found!!!", listener_id);
            None
        }
    })
}
