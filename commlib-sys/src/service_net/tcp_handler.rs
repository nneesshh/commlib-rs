use std::sync::Arc;

use message_io::network::{Endpoint, ResourceId};
use message_io::node::NodeHandler;

use crate::service_net::take_small_packet;
use crate::ServiceRs;

use super::{handle_close_conn_event, handle_message_event, net_packet::BUFFER_INITIAL_SIZE};
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
    listener_id.make_new_conn(PacketType::Server, hd, endpoint, netctrl, srv_net);
}

extern "C" fn on_connected_cb(tcp_client_ptr: *const TcpClient, hd: ConnId, os_addr: OsSocketAddr) {
    let cli = unsafe { &mut *(tcp_client_ptr as *mut TcpClient) };

    let id = ResourceId::from(hd.id);
    let sock_addr = os_addr.into_addr().unwrap();
    let endpoint = Endpoint::new(id, sock_addr);

    // make new conn
    cli.make_new_conn(PacketType::Server, hd, endpoint);
}

extern "C" fn on_message_cb(
    srv_net_ptr: *const Arc<ServiceNetRs>,
    hd: ConnId,
    input_data: *const u8,
    input_len: usize,
) {
    let srv_net = unsafe { &*srv_net_ptr };

    let mut remain: usize = input_len;
    let mut consumed: isize = 0;
    while remain > 0 {
        // 利用 buffer pkt 作为跨线程传递的数据缓存
        let len = std::cmp::min(remain, BUFFER_INITIAL_SIZE);
        let mut buffer_pkt = take_small_packet();

        let ptr = unsafe { input_data.offset(consumed) };
        buffer_pkt.append(ptr, len);

        // 在 srv_net 中运行
        let srv_net2 = srv_net.clone();
        let cb = move || {
            let conn_opt = srv_net2.get_conn(hd);
            if let Some(conn) = conn_opt {
                handle_message_event(srv_net2.as_ref(), &conn, buffer_pkt);
            } else {
                //
                log::error!("[on_message_cb][hd={}] conn not found!!!", hd);
            }
        };
        srv_net.run_in_service(Box::new(cb));

        //
        remain -= len;
        consumed += len as isize;
    }
}

extern "C" fn on_close_cb(srv_net_ptr: *const Arc<ServiceNetRs>, hd: ConnId) {
    let srv_net = unsafe { &*srv_net_ptr };

    // 在 srv_net 中运行
    let srv_net2 = srv_net.clone();
    let cb = move || {
        let conn_opt = srv_net2.get_conn(hd);
        if let Some(conn) = conn_opt {
            handle_close_conn_event(srv_net2.as_ref(), &conn);
        } else {
            //
            log::error!("[on_close_cb][hd={}] conn not found!!!", hd);
        }
    };
    srv_net.run_in_service(Box::new(cb));
}
