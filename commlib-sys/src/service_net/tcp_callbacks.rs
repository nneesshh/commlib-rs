use message_io::network::{Endpoint, NetworkController, ResourceId};
use opool::RefGuard;

use crate::service_net::NetPacketGuard;
use crate::{ServiceNetRs, ServiceRs};

use super::{ConnId, NetPacket, NetPacketPool, OsSocketAddr, PacketType, TcpConn, TcpServer};

/// Tcp server handler
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TcpServerHandler {
    pub on_listen: extern "C" fn(*const ServiceNetRs, *const TcpServer),
    pub on_accept: extern "C" fn(
        *const ServiceNetRs,
        *const TcpServer,
        *const NetworkController,
        ConnId,
        OsSocketAddr,
    ),

    pub on_message: extern "C" fn(*const ServiceNetRs, *const TcpServer, ConnId, *const u8, usize),
    pub on_close: extern "C" fn(*const ServiceNetRs, *const TcpServer, ConnId),
}

impl TcpServerHandler {
    /// Constructor
    pub fn new() -> TcpServerHandler {
        TcpServerHandler {
            on_listen: on_listen_cb,
            on_accept: on_accept_cb,

            on_message: on_server_message_cb,
            on_close: on_server_close_cb,
        }
    }
}

/// Tcp client handler
#[repr(C)]
pub struct TcpClientHandler {
    pub on_connect: extern "C" fn(*const ServiceNetRs, ConnId),
    pub on_message: extern "C" fn(*const ServiceNetRs, ConnId, *const u8, usize),
    pub on_close: extern "C" fn(*const ServiceNetRs, ConnId),
}

///
pub struct ServerCallbacks {
    pub srv: &'static dyn ServiceRs,
    pub conn_fn: Box<dyn Fn(ConnId) + Send + Sync>,
    pub pkt_fn: Box<dyn Fn(ConnId, NetPacketGuard) + Send + Sync>,
    pub stopped_cb: Box<dyn Fn(ConnId) + Send + Sync>,
}

impl ServerCallbacks {
    /// Constructor
    pub fn new<C, P, S>(
        srv: &'static dyn ServiceRs,
        conn_fn: C,
        pkt_fn: P,
        stopped_cb: S,
    ) -> ServerCallbacks
    where
        C: Fn(ConnId) + Send + Sync + 'static,
        P: Fn(ConnId, NetPacketGuard) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        ServerCallbacks {
            srv,
            conn_fn: Box::new(conn_fn),
            pkt_fn: Box::new(pkt_fn),
            stopped_cb: Box::new(stopped_cb),
        }
    }

    fn set_connection_callback<F>(&mut self, cb: F)
    where
        F: Fn(ConnId) + Send + Sync + 'static,
    {
        self.conn_fn = Box::new(cb);
    }

    fn set_message_callback<F>(&mut self, cb: F)
    where
        F: Fn(ConnId, NetPacketGuard) + Send + Sync + 'static,
    {
        self.pkt_fn = Box::new(cb);
    }
}

///
extern "C" fn on_listen_cb(srv_net: *const ServiceNetRs, tcp_server: *const TcpServer) {}

extern "C" fn on_accept_cb(
    srv_net: *const ServiceNetRs,
    tcp_server: *const TcpServer,
    network: *const NetworkController,
    hd: ConnId,
    os_sockaddr: OsSocketAddr,
) {
    let srv_net = unsafe { &*srv_net };
    let tcp_server = unsafe { &*tcp_server };
    let network = unsafe { &*network };

    let id = ResourceId::from(hd.id);
    let sockaddr = os_sockaddr.into_addr().unwrap();
    let endpoint = Endpoint::new(id, sockaddr);

    let conn = TcpConn::new(hd, endpoint, network);

    //
    {
        let mut conn_table_mut = srv_net.conn_table.write();
        (*conn_table_mut).insert(hd, conn);
    }

    // run callback in target srv
    let callbacks = tcp_server.callbacks_opt.as_ref().unwrap();
    callbacks.srv.run_in_service(Box::new(move || {
        (callbacks.conn_fn)(hd);
    }));
}

extern "C" fn on_server_encrypt_cb(
    srv_net: *const ServiceNetRs,
    tcp_server: *const TcpServer,
    hd: ConnId,
) {
}

extern "C" fn on_server_message_cb(
    srv_net: *const ServiceNetRs,
    tcp_server: *const TcpServer,
    hd: ConnId,
    input_data: *const u8,
    input_len: usize,
) {
    let srv_net = unsafe { &*srv_net };
    let tcp_server = unsafe { &*tcp_server };

    //
    {
        let conn_table = srv_net.conn_table.read();
        if let Some(conn) = conn_table.get(&hd) {
            // conn 循环处理 input
            let mut consumed = 0_usize;

            loop {
                let ptr = unsafe { input_data.offset(consumed as isize) };
                let (pkt_opt, remain) = conn.handle_read(ptr, input_len - consumed);
                if let Some(pkt) = pkt_opt {
                    let callbacks = tcp_server.callbacks_opt.as_ref().unwrap();
                    callbacks.srv.run_in_service(Box::new(move || {
                        (callbacks.pkt_fn)(hd, pkt);
                    }));
                }

                //
                consumed = input_len - remain;

                if 0 == remain {
                    break;
                }
            }
        } else {
            //
            log::error!("[on_server_message_cb][hd={:?}] not found!!!", hd);
        }
    }
}

extern "C" fn on_server_close_cb(
    srv_net: *const ServiceNetRs,
    tcp_server: *const TcpServer,
    hd: ConnId,
) {
    let srv_net = unsafe { &*srv_net };
    let tcp_server = unsafe { &*tcp_server };

    let mut packet_type = PacketType::Server;
    {
        let conn_table = srv_net.conn_table.read();
        if let Some(conn) = conn_table.get(&hd) {
            packet_type = conn.packet_type;
        }
    }

    // run callback in target srv
    let callbacks = tcp_server.callbacks_opt.as_ref().unwrap();
    callbacks.srv.run_in_service(Box::new(move || {
        (callbacks.stopped_cb)(hd);
    }));
}
