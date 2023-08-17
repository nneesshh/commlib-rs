use crate::ServiceNetRs;
use message_io::network::{Endpoint, NetworkController, ResourceId};
use std::net::SocketAddr;

use parking_lot::RwLock;

use super::{
    net_packet_pool::SMALL_PACKET_MAX_SIZE, take_packet, Buffer, NetPacketGuard, PacketType,
    TcpServer,
};

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
    pub fn send(&self, srv_net: &'static ServiceNetRs, data: &[u8]) {
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
    pub fn send_proto<M>(&self, srv_net: &'static ServiceNetRs, msg: &M)
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
    pub fn to_socket_addr(&self, srv_net: &'static ServiceNetRs) -> Option<SocketAddr> {
        //
        {
            let conn_table = srv_net.conn_table.read();
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
