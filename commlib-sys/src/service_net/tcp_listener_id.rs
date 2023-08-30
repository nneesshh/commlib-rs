use atomic::Atomic;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;

use message_io::network::Endpoint;
use message_io::node::NodeHandler;

use crate::{ServiceNetRs, ServiceRs};

use super::{ConnId, PacketReader, PacketType, ServerStatus, TcpConn, TcpServer};

/// Tcp server id
#[derive(Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct TcpListenerId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl TcpListenerId {
    /// Make new tcp conn with callbacks from tcp server
    pub fn make_new_conn(
        &self,
        packet_type: PacketType,
        hd: ConnId,
        endpoint: Endpoint,
        netctrl: &NodeHandler<()>,
        srv_net: &Arc<ServiceNetRs>,
    ) {
        let listener_id = *self;

        let srv_net2 = srv_net.clone();
        let netctrl2 = netctrl.clone();

        // insert tcp conn in srv net(同一线程便于观察 conn 生命周期)
        let cb = move || {
            let mut conn_opt: Option<Arc<TcpConn>> = None;
            {
                // check listener id
                let tcp_server_vec = srv_net2.tcp_server_vec.read();
                let mut tcp_server_opt: Option<&TcpServer> = None;
                for tcp_server in &*tcp_server_vec {
                    if tcp_server.listener_id == listener_id {
                        assert!(std::ptr::eq(&*tcp_server.srv_net, &*srv_net2));
                        tcp_server_opt = Some(tcp_server);
                    }
                }

                // 根据 tcp server 创建 tcp conn
                if let Some(tcp_server) = tcp_server_opt {
                    //
                    let srv = tcp_server.srv.clone();

                    //
                    let conn_fn = tcp_server.conn_fn.clone();
                    let pkt_fn = tcp_server.pkt_fn.clone();
                    let close_fn = tcp_server.close_fn.clone();

                    let conn = Arc::new(TcpConn {
                        //
                        packet_type: Atomic::new(PacketType::Server),
                        hd,

                        //
                        endpoint,
                        netctrl: netctrl2.clone(),

                        //
                        closed: Atomic::new(false),

                        //
                        srv: srv.clone(),
                        srv_net: srv_net2.clone(),

                        //
                        conn_fn,
                        pkt_fn,
                        close_fn: RwLock::new(close_fn),

                        //
                        pkt_reader: PacketReader::new(packet_type),
                    });

                    //
                    conn_opt = Some(conn);
                }
            };

            if let Some(conn) = conn_opt {
                // add conn to service net
                srv_net2.insert_conn(conn.hd, &conn);

                // trigger conn_fn
                conn.run_conn_fn();
            }
        };
        srv_net.run_in_service(Box::new(cb));
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
