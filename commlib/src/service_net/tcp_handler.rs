use std::sync::Arc;

use net_packet::NetPacketGuard;

use crate::{ServiceNetRs, ServiceRs};

use super::connector::{on_connector_connect_err, on_connector_connect_ok};
use super::listener::{on_listener_accept, on_listener_listen};
use super::tcp_conn_manager::{on_connection_closed, on_connection_read_data};
use super::{ConnId, ListenerId, OsSocketAddr};

///
#[repr(transparent)]
pub struct InputBuffer {
    pub data: NetPacketGuard,
}

/* FFI-safe */
/*
pub type OnListenFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ListenerId, OsSocketAddr);
pub type OnAcceptFuncType =
    extern "C" fn(*const Arc<ServiceNetRs>, ListenerId, ConnId, OsSocketAddr);
pub type OnConnectOkFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId, OsSocketAddr);
pub type OnConnectErrFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId);
pub type OnInputFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId, *const Arc<InputBuffer>);
pub type OnCloseFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId);
 */

///
pub type OnListenFuncType = fn(&Arc<ServiceNetRs>, ListenerId, OsSocketAddr);

///
pub type OnAcceptFuncType = fn(&Arc<ServiceNetRs>, ListenerId, ConnId, OsSocketAddr);

///
pub type OnConnectOkFuncType = fn(&Arc<ServiceNetRs>, ConnId, OsSocketAddr);

///
pub type OnConnectErrFuncType = fn(&Arc<ServiceNetRs>, ConnId);

///
pub type OnInputFuncType = fn(&Arc<ServiceNetRs>, ConnId, InputBuffer);

///
pub type OnCloseFuncType = fn(&Arc<ServiceNetRs>, ConnId);

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
fn on_listen_cb(srv_net: &Arc<ServiceNetRs>, listener_id: ListenerId, os_addr: OsSocketAddr) {
    let sock_addr = os_addr.into_addr().unwrap();

    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    //
    on_listener_listen(srv_net.as_ref(), listener_id, sock_addr);
}

fn on_accept_cb(
    srv_net: &Arc<ServiceNetRs>,
    listener_id: ListenerId,
    hd: ConnId,
    os_addr: OsSocketAddr,
) {
    let sock_addr = os_addr.into_addr().unwrap();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_listener_accept(srv_net2.as_ref(), listener_id, hd, sock_addr);
    };
    srv_net.run_in_service(Box::new(func));
}

fn on_connect_ok_cb(srv_net: &Arc<ServiceNetRs>, hd: ConnId, os_addr: OsSocketAddr) {
    let sock_addr = os_addr.into_addr().unwrap();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connector_connect_ok(srv_net2.as_ref(), hd, sock_addr);
    };
    srv_net.run_in_service(Box::new(func));
}

fn on_connect_err_cb(srv_net: &Arc<ServiceNetRs>, hd: ConnId) {
    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connector_connect_err(srv_net2.as_ref(), hd);
    };
    srv_net.run_in_service(Box::new(func));
}

fn on_input_cb(srv_net: &Arc<ServiceNetRs>, hd: ConnId, input_buffer: InputBuffer) {
    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connection_read_data(srv_net2.as_ref(), hd, input_buffer.data);
    };
    srv_net.run_in_service(Box::new(func));
}

fn on_close_cb(srv_net: &Arc<ServiceNetRs>, hd: ConnId) {
    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connection_closed(srv_net2.as_ref(), hd);
    };
    srv_net.run_in_service(Box::new(func));
}
