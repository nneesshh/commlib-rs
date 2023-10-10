use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs};

use super::connector::{on_connect_failed, on_connect_success};
use super::take_packet;
use super::tcp_conn_manager::{on_connection_closed, on_connection_read_data};
use super::tcp_server_manager::tcp_server_make_new_conn;
use super::{ConnId, OsSocketAddr, PacketType, TcpListenerId, TcpServer};

///
pub type OnListenFuncType = extern "C" fn(*mut TcpServer, TcpListenerId, OsSocketAddr);

///
pub type OnAcceptFuncType =
    extern "C" fn(*const Arc<ServiceNetRs>, TcpListenerId, ConnId, OsSocketAddr);

///
pub type OnConnectOkFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId, OsSocketAddr);

///
pub type OnConnectErrFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId);

///
pub type OnInputFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId, *const u8, usize);

///
pub type OnCloseFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId);

/// Tcp server handler
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TcpHandler {
    pub on_listen: OnListenFuncType,
    pub on_accept: OnAcceptFuncType,

    pub on_connect_ok: OnConnectOkFuncType,
    pub on_connect_err: OnConnectErrFuncType,

    pub on_input: OnInputFuncType,
    pub on_close: OnCloseFuncType,
}

impl TcpHandler {
    /// Constructor
    pub fn new() -> Self {
        Self {
            on_listen: on_listen_cb,
            on_accept: on_accept_cb,

            on_connect_ok: on_connect_ok_cb,
            on_connect_err: on_connect_err_cb,

            on_input: on_input_cb,
            on_close: on_close_cb,
        }
    }
}

///
extern "C" fn on_listen_cb(
    tcp_server_ptr: *mut TcpServer,
    listener_id: TcpListenerId,
    os_addr: OsSocketAddr,
) {
    let tcp_server = unsafe { &mut *tcp_server_ptr };

    // trigger listen_fn in main service
    let sock_addr = os_addr.into_addr().unwrap();
    listener_id.run_listen_fn(tcp_server, sock_addr);
}

extern "C" fn on_accept_cb(
    srv_net_ptr: *const Arc<ServiceNetRs>,
    listener_id: TcpListenerId,
    hd: ConnId,
    os_addr: OsSocketAddr,
) {
    let srv_net = unsafe { &*srv_net_ptr };
    let sock_addr = os_addr.into_addr().unwrap();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        // make new conn
        tcp_server_make_new_conn(&srv_net2, listener_id, PacketType::Server, hd, sock_addr);
    };
    srv_net.run_in_service(Box::new(func));
}

extern "C" fn on_connect_ok_cb(
    srv_net_ptr: *const Arc<ServiceNetRs>,
    hd: ConnId,
    os_addr: OsSocketAddr,
) {
    let srv_net = unsafe { &*srv_net_ptr };
    let sock_addr = os_addr.into_addr().unwrap();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connect_success(srv_net2.as_ref(), hd, sock_addr);
    };
    srv_net.run_in_service(Box::new(func));
}

extern "C" fn on_connect_err_cb(srv_net_ptr: *const Arc<ServiceNetRs>, hd: ConnId) {
    let srv_net = unsafe { &*srv_net_ptr };

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connect_failed(srv_net2.as_ref(), hd);
    };
    srv_net.run_in_service(Box::new(func));
}

extern "C" fn on_input_cb(
    srv_net_ptr: *const Arc<ServiceNetRs>,
    hd: ConnId,
    input_data: *const u8,
    input_len: usize,
) {
    let srv_net = unsafe { &*srv_net_ptr };

    // 利用 buffer pkt 作为跨线程传递的数据缓存 （需要 TcpConn 设置 leading_filed_size）
    let mut input_buffer = take_packet(input_len, 0);
    input_buffer.append(input_data, input_len);

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connection_read_data(srv_net2.as_ref(), hd, input_buffer);
    };
    srv_net.run_in_service(Box::new(func));
}

extern "C" fn on_close_cb(srv_net_ptr: *const Arc<ServiceNetRs>, hd: ConnId) {
    let srv_net = unsafe { &*srv_net_ptr };

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connection_closed(srv_net2.as_ref(), hd);
    };
    srv_net.run_in_service(Box::new(func));
}
