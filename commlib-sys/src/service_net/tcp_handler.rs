use std::sync::Arc;

use message_io::network::{Endpoint, ResourceId};
use message_io::node::NodeHandler;

use super::take_packet;
use super::tcp_client_manager::tcp_client_make_new_conn;
use super::tcp_conn_manager::{handle_close_conn_event, handle_message_event};
use super::tcp_server_manager::tcp_server_make_new_conn;
use super::{ConnId, OsSocketAddr, PacketType, ServiceNetRs, TcpClient, TcpListenerId, TcpServer};

///
pub type OnListenFuncType = extern "C" fn(*const TcpServer, TcpListenerId, OsSocketAddr);

///
pub type OnAcceptFuncType = extern "C" fn(
    *const Arc<ServiceNetRs>,
    *const NodeHandler<()>,
    TcpListenerId,
    ConnId,
    OsSocketAddr,
);

///
pub type OnConnectedFuncType = extern "C" fn(*const TcpClient, ConnId, OsSocketAddr);

///
pub type OnMessageFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId, *const u8, usize);

///
pub type OnCloseFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId);

/// Tcp server handler
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TcpHandler {
    pub on_listen: OnListenFuncType,
    pub on_accept: OnAcceptFuncType,

    pub on_connected: OnConnectedFuncType,

    pub on_message: OnMessageFuncType,
    pub on_close: OnCloseFuncType,
}

impl TcpHandler {
    /// Constructor
    pub fn new() -> TcpHandler {
        TcpHandler {
            on_listen: on_listen_cb,
            on_accept: on_accept_cb,

            on_connected: on_connected_cb,

            on_message: on_message_cb,
            on_close: on_close_cb,
        }
    }
}

///
extern "C" fn on_listen_cb(
    tcp_server_ptr: *const TcpServer,
    listener_id: TcpListenerId,
    os_addr: OsSocketAddr,
) {
    let tcp_server = unsafe { &mut *(tcp_server_ptr as *mut TcpServer) };

    // trigger listen_fn in main service
    let sock_addr = os_addr.into_addr().unwrap();
    listener_id.run_listen_fn(tcp_server, sock_addr);
}

extern "C" fn on_accept_cb(
    srv_net_ptr: *const Arc<ServiceNetRs>,
    netctrl_ptr: *const NodeHandler<()>,
    listener_id: TcpListenerId,
    hd: ConnId,
    os_addr: OsSocketAddr,
) {
    let srv_net = unsafe { &*srv_net_ptr };
    let netctrl = unsafe { &*netctrl_ptr };

    let id = ResourceId::from(hd.id);
    let sock_addr = os_addr.into_addr().unwrap();
    let endpoint = Endpoint::new(id, sock_addr);

    // make new conn
    tcp_server_make_new_conn(
        srv_net,
        listener_id,
        PacketType::Server,
        hd,
        endpoint,
        netctrl,
    );
}

extern "C" fn on_connected_cb(tcp_client_ptr: *const TcpClient, hd: ConnId, os_addr: OsSocketAddr) {
    let cli = unsafe { &mut *(tcp_client_ptr as *mut TcpClient) };

    let id = ResourceId::from(hd.id);
    let sock_addr = os_addr.into_addr().unwrap();
    let endpoint = Endpoint::new(id, sock_addr);

    // make new conn
    tcp_client_make_new_conn(cli, PacketType::Server, hd, endpoint);
}

extern "C" fn on_message_cb(
    srv_net_ptr: *const Arc<ServiceNetRs>,
    hd: ConnId,
    input_data: *const u8,
    input_len: usize,
) {
    let srv_net = unsafe { &*srv_net_ptr };

    // 利用 buffer pkt 作为跨线程传递的数据缓存 （需要 TcpConn 设置 leading_filed_size）
    let mut buffer_pkt = take_packet(input_len, 0);
    buffer_pkt.append(input_data, input_len);

    //
    handle_message_event(srv_net, hd, buffer_pkt);
}

extern "C" fn on_close_cb(srv_net_ptr: *const Arc<ServiceNetRs>, hd: ConnId) {
    let srv_net = unsafe { &*srv_net_ptr };

    //
    handle_close_conn_event(srv_net, hd);
}
