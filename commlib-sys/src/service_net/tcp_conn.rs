use super::net_packet::PacketType;

use crate::G_SERVICE_NET;

use message_io::network::{Endpoint, NetworkController, ResourceId};
use std::net::SocketAddr;

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
    pub fn send(&self, data: &[u8]) {
        //
        {
            let conn_table = G_SERVICE_NET.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                tcp_conn.send(data);
            } else {
                log::error!("[hd={:?}] send failed -- hd not found!!!", *self);
            }
        }
    }

    ///
    #[inline(always)]
    pub fn send_proto<M>(&self, msg: &M)
    where
        M: prost::Message,
    {
        let vec = msg.encode_to_vec();

        //
        {
            let conn_table = G_SERVICE_NET.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                tcp_conn.send(vec.as_slice());
            } else {
                log::error!("[hd={:?}] send_proto failed -- hd not found!!!", *self);
            }
        }
    }

    ///
    pub fn to_socket_addr(&self) -> Option<SocketAddr> {
        //
        {
            let conn_table = G_SERVICE_NET.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                let id = ResourceId::from(self.id);
                Some(tcp_conn.endpoint.addr())
            } else {
                log::error!("[hd={:?}] to_socket_addr failed -- hd not found!!!", *self);
                None
            }
        }
    }
}

impl From<usize> for ConnId {
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}

/// Tcp connection
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct TcpConn {
    pub packet_type: PacketType,
    pub hd: ConnId,

    //
    pub endpoint: Endpoint,
    pub netctrl_id: usize,
}

impl TcpConn {
    ///
    pub fn new(hd: ConnId, endpoint: Endpoint, netctrl: &NetworkController) -> TcpConn {
        let packet_type = PacketType::Server;
        let netctrl_id = netctrl as *const NetworkController as usize;

        TcpConn {
            packet_type,
            hd,
            endpoint,
            netctrl_id,
        }
    }

    ///
    pub fn send(&self, data: &[u8]) {
        log::debug!("[hd={:?}] send data ...", self.hd);

        let netctrl = unsafe { &*(self.netctrl_id as *const NetworkController) };
        netctrl.send(self.endpoint, data);
    }
}
