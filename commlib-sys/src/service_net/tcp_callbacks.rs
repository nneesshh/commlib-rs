use super::net_packet::NetPacket;
use super::net_packet_pool::take_packet;
use super::net_packet_pool::NetPacketPool;
use super::tcp_conn::*;
use super::OsSocketAddr;
use crate::service_net::net_packet::PacketType;
use crate::service_net::TcpServer;
use crate::{ServiceNetRs, ServiceRs};
use message_io::network::{Endpoint, NetworkController, ResourceId};
use opool::RefGuard;

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
    pub srv: Option<&'static dyn ServiceRs>,
    pub conn_fn: Box<dyn Fn(ConnId) + Send + Sync + 'static>,
    pub msg_fn:
        Box<dyn Fn(ConnId, RefGuard<'static, NetPacketPool, NetPacket>) + Send + Sync + 'static>,
    pub stopped_cb: Box<dyn Fn(ConnId) + Send + Sync + 'static>,
}

impl ServerCallbacks {
    /// Constructor
    pub fn new() -> ServerCallbacks {
        ServerCallbacks {
            srv: None,
            conn_fn: Box::new(|_1| {}),
            msg_fn: Box::new(|_1, _2| {}),

            stopped_cb: Box::new(|_1| {}),
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
        F: Fn(ConnId, RefGuard<'static, NetPacketPool, NetPacket>) + Send + Sync + 'static,
    {
        self.msg_fn = Box::new(cb);
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
    if let Some(srv) = tcp_server.callbacks.srv {
        srv.run_in_service(Box::new(move || {
            (tcp_server.callbacks.conn_fn)(hd);
        }));
    } else {
        std::unreachable!();
    }
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
    data: *const u8,
    len: usize,
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

    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    let pkt = take_packet(len, packet_type, slice);

    // run callback in target srv
    if let Some(srv) = tcp_server.callbacks.srv {
        srv.run_in_service(Box::new(move || {
            (tcp_server.callbacks.msg_fn)(hd, pkt);
        }));
    } else {
        std::unreachable!();
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
    if let Some(srv) = tcp_server.callbacks.srv {
        srv.run_in_service(Box::new(move || {
            (tcp_server.callbacks.stopped_cb)(hd);
        }));
    } else {
        std::unreachable!();
    }
}
