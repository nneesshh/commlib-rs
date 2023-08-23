use std::net::SocketAddr;
use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs};
use message_io::network::ResourceId;

use super::NetPacketGuard;

/// Connection id
#[derive(Debug, Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct ConnId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl ConnId {
    ///
    #[inline(always)]
    pub fn send(&self, srv_net: &Arc<ServiceNetRs>, data: &[u8]) {
        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                tcp_conn.send(data);
            } else {
                log::error!("[hd={:?}] send failed -- hd not found!!!", *self);
            }
        }
    }

    ///
    #[inline(always)]
    pub fn send_proto<M>(&self, srv_net: &Arc<ServiceNetRs>, msg: &M)
    where
        M: prost::Message,
    {
        let vec = msg.encode_to_vec();

        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                tcp_conn.send(vec.as_slice());
            } else {
                log::error!("[hd={:?}] send_proto failed -- hd not found!!!", *self);
            }
        }
    }

    ///
    pub fn to_socket_addr(&self, srv_net: &Arc<ServiceNetRs>) -> Option<SocketAddr> {
        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                Some(tcp_conn.endpoint.addr())
            } else {
                log::error!("[hd={:?}] to_socket_addr failed -- hd not found!!!", *self);
                None
            }
        }
    }

    /// call conn_fn
    pub fn run_conn_fn(&self, srv: &Arc<dyn ServiceRs>, srv_net: &Arc<ServiceNetRs>) {
        let hd = *self;

        let f_opt = {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                Some(conn.close_fn.clone())
            } else {
                None
            }
        };

        //
        srv.run_in_service(Box::new(move || {
            if let Some(f) = f_opt {
                (f)(hd);
            } else {
                log::error!("[hd={:?}][run_conn_fn] failed!!!", hd);
            }
        }));
    }

    /// call pkt_fn
    pub fn run_pkt_fn(
        &self,
        srv: &Arc<dyn ServiceRs>,
        srv_net: &Arc<ServiceNetRs>,
        pkt: NetPacketGuard,
    ) {
        let hd = *self;

        let f_opt = {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                Some(conn.pkt_fn.clone())
            } else {
                None
            }
        };

        //
        srv.run_in_service(Box::new(move || {
            if let Some(f) = f_opt {
                (f)(hd, pkt);
            } else {
                log::error!("[hd={:?}][run_pkt_fn] failed!!!", hd);
            }
        }));
    }

    /// call close_fn
    pub fn run_close_fn(&self, srv: &Arc<dyn ServiceRs>, srv_net: &Arc<ServiceNetRs>) {
        let hd = *self;

        let f_opt = {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                Some(conn.close_fn.clone())
            } else {
                None
            }
        };

        //
        srv.run_in_service(Box::new(move || {
            if let Some(f) = f_opt {
                (f)(hd);
            } else {
                log::error!("[hd={:?}][run_close_fn] failed!!!", hd);
            }
        }));
    }
}

impl From<usize> for ConnId {
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}
