use atomic::{Atomic, Ordering};
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use net_packet::NetPacketGuard;

use crate::ServiceRs;

use super::low_level_network::MessageIoNetwork;
use super::tcp_conn_manager::on_connection_closed;
use super::{ConnId, PacketType, ServiceNetRs};

/// Tcp connection: all fields are public for easy construct
pub struct TcpConn {
    //
    pub hd: ConnId,

    //
    pub sock_addr: SocketAddr,

    //
    pub packet_type: Atomic<PacketType>,
    pub closed: Atomic<bool>,

    //
    pub netctrl_opt: Option<Arc<MessageIoNetwork>>,
    pub srv_net_opt: Option<Arc<ServiceNetRs>>,

    // 运行于 srv_net 线程：处理连接事件
    pub connection_establish_fn: Box<dyn Fn(Arc<TcpConn>) + Send + Sync>,

    // 运行于 srv_net 线程：对 input buffer 数据进行分包处理
    pub connection_read_fn: Box<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>,

    // 运行于 srv_net 线程：处理连接断开事件
    pub connection_lost_fn: RwLock<Arc<dyn Fn(ConnId) + Send + Sync>>,
}

impl TcpConn {
    /// For debug only
    pub fn new(hd: ConnId) -> Self {
        Self {
            //
            hd,

            //
            sock_addr: SocketAddr::from_str("127.0.0.1:0").unwrap(),

            //
            packet_type: Atomic::new(PacketType::Server),
            closed: Atomic::new(false),

            //
            netctrl_opt: None,
            srv_net_opt: None,

            //
            connection_establish_fn: Box::new(|_1| {}),
            connection_read_fn: Box::new(|_1, _2| {}),
            connection_lost_fn: RwLock::new(Arc::new(|_1| {})),
        }
    }

    /// low level close
    #[inline(always)]
    pub fn close(&self) {
        let hd = self.hd;

        // already closed?
        if self.is_closed() {
            log::error!("[hd={}] already closed!!!", hd);
            return;
        }

        //
        //log::info!("[hd={}] low level close", hd);
        self.netctrl_opt.as_ref().unwrap().close(hd);

        let srv_net2 = self.srv_net_opt.as_ref().unwrap().clone();

        // trigger connetion closed event
        // 运行于 srv_net 线程 (不管当前是否已经位于 srv_net 线程中，始终投递)
        self.srv_net_opt
            .as_ref()
            .unwrap()
            .run_in_service(Box::new(move || {
                on_connection_closed(&srv_net2, hd);
            }));
    }

    ///
    #[inline(always)]
    pub fn send_buffer(&self, buffer: NetPacketGuard) {
        let hd = self.hd;
        self.netctrl_opt
            .as_ref()
            .unwrap()
            .send_buffer(hd, self.sock_addr, buffer);
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
