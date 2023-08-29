use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;

use message_io::network::Endpoint;
use message_io::node::NodeHandler;

use crate::ServiceNetRs;

use super::{ConnId, PacketReader, PacketType, ServerStatus, TcpConn, TcpServer};

/// Tcp server id
#[derive(Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct TcpListenerId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl TcpListenerId {
    /// Make conn with callbacks from tcp server
    pub fn make_new_conn(
        &self,
        packet_type: PacketType,
        hd: ConnId,
        endpoint: Endpoint,
        netctrl: &NodeHandler<()>,
        srv_net: &Arc<ServiceNetRs>,
    ) -> bool {
        let listener_id = *self;

        //
        let tcp_server_vec = srv_net.tcp_server_vec.read();
        for tcp_server in &*tcp_server_vec {
            if tcp_server.listener_id == listener_id {
                assert!(std::ptr::eq(&*tcp_server.srv_net, &**srv_net));

                //
                let srv = tcp_server.srv.clone();
                let srv_net = tcp_server.srv_net.clone();

                //
                let conn_fn = tcp_server.conn_fn.clone();
                let pkt_fn = tcp_server.pkt_fn.clone();
                let close_fn = tcp_server.close_fn.clone();

                let conn = Arc::new(TcpConn {
                    //
                    packet_type: PacketType::Server.into(),
                    hd,

                    //
                    endpoint,
                    netctrl: netctrl.clone(),

                    //
                    closed: false.into(),

                    //
                    srv: srv.clone(),
                    srv_net: srv_net.clone(),

                    //
                    conn_fn,
                    pkt_fn,
                    close_fn: RwLock::new(close_fn),

                    //
                    pkt_reader: PacketReader::new(packet_type),
                });

                // add conn to service net
                srv_net.insert_conn(conn.hd, &conn);
                return true;
            }
        }

        //
        false
    }

    /// Trigger listen_fn of tcp server
    pub fn run_listen_fn(&self, tcp_server: &mut TcpServer, sock_addr: SocketAddr) {
        let srv = tcp_server.srv.as_ref();
        let listener_id = *self;

        // update listener id
        tcp_server.listener_id = listener_id;

        // 状态：Running
        tcp_server.set_status(ServerStatus::Running);

        //
        let listen_fn_opt = Some(tcp_server.listen_fn.clone());
        let status = tcp_server.status();
        srv.run_in_service(Box::new(move || {
            if let Some(listen_fn) = listen_fn_opt {
                listen_fn(sock_addr, status);
            } else {
                log::error!("[listen_id={}][run_listen_fn] failed!!!", listener_id);
            }
        }));
    }
}

impl From<usize> for TcpListenerId {
    #[inline(always)]
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}

impl std::fmt::Display for TcpListenerId {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
