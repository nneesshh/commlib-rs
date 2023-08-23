use std::sync::Arc;

use message_io::network::Endpoint;
use message_io::node::NodeHandler;

use crate::ServiceRs;

use super::{ConnId, NetPacketGuard, PacketReader, PacketType};

/// Tcp connection
#[repr(C)]
pub struct TcpConn {
    //
    pub packet_type: PacketType,
    pub hd: ConnId,

    //
    pub endpoint: Endpoint,
    pub netctrl: NodeHandler<()>,

    //
    pub srv: Arc<dyn ServiceRs>,
    pub conn_fn: Arc<dyn Fn(ConnId) + Send + Sync>,
    pub pkt_fn: Arc<dyn Fn(ConnId, NetPacketGuard) + Send + Sync>,
    pub close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,

    //
    pkt_reader: PacketReader,
}

impl TcpConn {
    ///
    pub fn new(
        hd: ConnId,
        endpoint: Endpoint,
        netctrl: &NodeHandler<()>,
        srv: &Arc<dyn ServiceRs>,
    ) -> TcpConn {
        let packet_type = PacketType::Server;

        TcpConn {
            packet_type,
            hd,

            endpoint,
            netctrl: netctrl.clone(),

            srv: srv.clone(),
            conn_fn: Arc::new(|_hd| {}),
            pkt_fn: Arc::new(|_hd, _pkt| {}),
            close_fn: Arc::new(|_hd| {}),

            pkt_reader: PacketReader::new(packet_type),
        }
    }

    ///
    #[inline(always)]
    pub fn handle_read(&self, data: *const u8, len: usize) -> (Option<NetPacketGuard>, usize) {
        self.pkt_reader.read(data, len)
    }

    ///
    pub fn send(&self, data: &[u8]) {
        log::debug!("[hd={:?}] send data ...", self.hd);

        self.netctrl.network().send(self.endpoint, data);
    }
}
