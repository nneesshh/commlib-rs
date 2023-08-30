use atomic::{Atomic, Ordering};
use parking_lot::RwLock;
use std::sync::Arc;

use message_io::network::Endpoint;
use message_io::node::NodeHandler;

use crate::ServiceRs;

use super::packet_reader::PacketResult;
use super::{ConnId, NetPacketGuard, PacketReader, PacketType, ServiceNetRs};

/// Tcp connection: all fields are public for easy construct
pub struct TcpConn {
    //
    pub packet_type: Atomic<PacketType>,
    pub hd: ConnId,

    //
    pub endpoint: Endpoint,
    pub netctrl: NodeHandler<()>,

    //
    pub closed: Atomic<bool>,

    //
    pub srv: Arc<dyn ServiceRs>,
    pub srv_net: Arc<ServiceNetRs>,

    //
    pub conn_fn: Arc<dyn Fn(ConnId) + Send + Sync>,
    pub pkt_fn: Arc<dyn Fn(ConnId, NetPacketGuard) + Send + Sync>,
    pub close_fn: RwLock<Arc<dyn Fn(ConnId) + Send + Sync>>,

    //
    pub pkt_reader: PacketReader,
}

impl TcpConn {
    ///
    #[inline(always)]
    pub fn handle_read(&self, data: *const u8, len: usize) -> PacketResult {
        self.pkt_reader.read(data, len)
    }

    /// low level close
    #[inline(always)]
    pub fn close(&self) {
        log::info!("[hd={}] low level close", self.hd);
        self.netctrl.network().remove(self.endpoint.resource_id());
    }

    ///
    #[inline(always)]
    pub fn send(&self, data: &[u8]) {
        log::debug!("[hd={}] send data ...", self.hd);

        self.netctrl.network().send(self.endpoint, data);
    }

    /// call conn_fn
    pub fn run_conn_fn(&self) {
        let hd = self.hd;
        let f = self.conn_fn.clone();

        //
        self.srv.run_in_service(Box::new(move || {
            (f)(hd);
        }));
    }

    /// call pkt_fn
    pub fn run_pkt_fn(&self, pkt: NetPacketGuard) {
        let hd = self.hd;
        let f = self.pkt_fn.clone();

        //
        self.srv.run_in_service(Box::new(move || {
            (f)(hd, pkt);
        }));
    }

    /// call close_fn
    pub fn run_close_fn(&self) {
        let hd = self.hd;

        let f: Arc<dyn Fn(ConnId) + Send + Sync>;
        {
            let close_fn = self.close_fn.read();
            f = (*close_fn).clone();
        }

        // 标记关闭
        self.closed.store(true, Ordering::Relaxed);

        //
        self.srv.run_in_service(Box::new(move || {
            (f)(hd);
        }));
    }

    ///
    #[inline(always)]
    pub fn packet_type(&self) -> PacketType {
        self.packet_type.load(Ordering::Relaxed)
    }

    ///
    #[inline(always)]
    pub fn set_packet_type(&self, packet_type: PacketType) {
        self.packet_type.store(packet_type, Ordering::Relaxed);
    }
}
