use atomic::{Atomic, Ordering};
use parking_lot::RwLock;
use std::sync::Arc;

use message_io::network::Endpoint;
use message_io::node::NodeHandler;

use crate::ServiceRs;

use super::tcp_conn_manager::on_connection_closed;
use super::{ConnId, ServiceNetRs};
use super::{NetPacketGuard, PacketType};

/// Tcp connection: all fields are public for easy construct
pub struct TcpConn {
    //
    pub hd: ConnId,

    //
    pub endpoint: Endpoint,
    pub netctrl: NodeHandler<()>,

    //
    pub packet_type: Atomic<PacketType>,
    pub closed: Atomic<bool>,

    //
    pub srv: Arc<dyn ServiceRs>,
    pub srv_net: Arc<ServiceNetRs>,

    // 运行于 srv_net 线程：处理连接事件
    pub connection_establish_fn: Box<dyn Fn(Arc<TcpConn>) + Send + Sync>,

    // 运行于 srv_net 线程：对 input buffer 数据进行分包处理
    pub connection_read_fn: Box<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>,

    // 运行于 srv_net 线程：处理连接断开事件
    pub connection_lost_fn: RwLock<Arc<dyn Fn(ConnId) + Send + Sync>>,
}

impl TcpConn {
    /// low level close
    #[inline(always)]
    pub fn close(&self) {
        log::info!("[hd={}] low level close", self.hd);
        self.netctrl.network().remove(self.endpoint.resource_id());

        let srv_net2 = self.srv_net.clone();
        let hd = self.hd;

        // trigger connetion closed event
        // 运行于 srv_net 线程 (不管当前是否已经位于 srv_net 线程中，始终投递)
        self.srv_net.run_in_service(Box::new(move || {
            on_connection_closed(&srv_net2, hd);
        }));
    }

    ///
    #[inline(always)]
    pub fn send(&self, data: &[u8]) {
        log::debug!("[hd={}] send data ...", self.hd);

        self.netctrl.network().send(self.endpoint, data);
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

    ///
    #[inline(always)]
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    ///
    #[inline(always)]
    pub fn set_closed(&self, is_closed: bool) {
        self.closed.store(is_closed, Ordering::Relaxed);
    }
}
