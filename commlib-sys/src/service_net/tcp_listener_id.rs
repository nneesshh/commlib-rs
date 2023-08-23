use std::net::SocketAddr;
use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs};

use super::{TcpConn, TcpServer};

/// Tcp server id
#[derive(Debug, Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct TcpListenerId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl TcpListenerId {
    /// call listen_fn of tcp server
    pub fn service(&self, srv_net: &Arc<ServiceNetRs>) -> Option<Arc<dyn ServiceRs>> {
        let listener_id = *self;
        let tcp_server_vec = srv_net.tcp_server_vec.read();
        for tcp_server in &(*tcp_server_vec) {
            if tcp_server.listener_id == listener_id {
                return Some(tcp_server.srv.clone());
            }
        }
        None
    }

    /// make conn callbacks from tcp server
    pub fn bind_callbacks(&self, conn: &mut TcpConn, srv_net: &Arc<ServiceNetRs>) {
        let listener_id = *self;

        //
        let tcp_server_vec = srv_net.tcp_server_vec.read();
        for tcp_server in &*tcp_server_vec {
            if tcp_server.listener_id == listener_id {
                //
                conn.conn_fn = tcp_server.conn_fn.clone();
                conn.pkt_fn = tcp_server.pkt_fn.clone();
                conn.close_fn = tcp_server.close_fn.clone();
            };
        }
    }

    /// trigger listen_fn of tcp server
    pub fn run_listen_fn(&self, tcp_server: &mut TcpServer, sock_addr: SocketAddr) {
        let srv = tcp_server.srv.as_ref();
        let listener_id = *self;

        // update listener id
        tcp_server.listener_id = listener_id;

        //
        let listen_fn_opt = Some(tcp_server.listen_fn.clone());
        srv.run_in_service(Box::new(move || {
            if let Some(listen_fn) = listen_fn_opt {
                listen_fn(sock_addr);
            } else {
                log::error!("[listen_id={:?}][run_listen_fn] failed!!!", listener_id);
            }
        }));
    }
}

impl From<usize> for TcpListenerId {
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}
