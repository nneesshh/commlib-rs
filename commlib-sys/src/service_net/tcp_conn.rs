use crate::ServiceNetRs;
use message_io::network::{Endpoint, NetworkController, ResourceId};
use std::net::SocketAddr;

use parking_lot::RwLock;

use super::{take_packet, Buffer, ConnId, NetPacketGuard, PacketReader, PacketType, TcpServer};

/// Tcp connection
#[repr(C)]
pub struct TcpConn {
    //
    pub packet_type: PacketType,
    pub hd: ConnId,

    //
    pub endpoint: Endpoint,
    pub netctrl_id: usize,

    //
    pkt_reader: PacketReader,
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

        let netctrl = unsafe { &*(self.netctrl_id as *const NetworkController) };
        netctrl.send(self.endpoint, data);
    }
}
