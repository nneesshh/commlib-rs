use std::sync::Arc;

use message_io::network::{Endpoint, ResourceId};
use message_io::node::NodeHandler;

use super::{ConnId, OsSocketAddr, ServiceNetRs, TcpConn, TcpListenerId, TcpServer};

///
pub type OnListenFuncType =
    extern "C" fn(*const Arc<ServiceNetRs>, *const TcpServer, TcpListenerId, OsSocketAddr);

///
pub type OnAcceptFuncType = extern "C" fn(
    *const Arc<ServiceNetRs>,
    *const NodeHandler<()>,
    TcpListenerId,
    ConnId,
    OsSocketAddr,
);

///
pub type OnConnectFuncType = extern "C" fn(*const Arc<ServiceNetRs>, ConnId);

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

    pub on_connect: OnConnectFuncType,

    pub on_message: OnMessageFuncType,
    pub on_close: OnCloseFuncType,
}

impl TcpHandler {
    /// Constructor
    pub fn new() -> TcpHandler {
        TcpHandler {
            on_listen: on_listen_cb,
            on_accept: on_accept_cb,

            on_connect: on_connect_cb,

            on_message: on_message_cb,
            on_close: on_close_cb,
        }
    }
}

///
extern "C" fn on_listen_cb(
    srv_net_ptr: *const Arc<ServiceNetRs>,
    tcp_server_ptr: *const TcpServer,
    listener_id: TcpListenerId,
    os_addr: OsSocketAddr,
) {
    let srv_net = unsafe { &*srv_net_ptr };
    let tcp_server = unsafe { &mut *(tcp_server_ptr as *mut TcpServer) };

    // trigger listen_cb
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

    let srv = listener_id.service(srv_net).unwrap();

    // insert new conn
    {
        let mut conn = TcpConn::new(hd, endpoint, netctrl, &srv);
        listener_id.bind_callbacks(&mut conn, srv_net);

        {
            let mut conn_table_mut = srv_net.conn_table.write();
            (*conn_table_mut).insert(hd, conn);
        }
    }

    // trigger conn_fn
    hd.run_conn_fn(&srv, srv_net);
}

extern "C" fn on_connect_cb(srv_net: *const Arc<ServiceNetRs>, hd: ConnId) {}

extern "C" fn on_message_cb(
    srv_net_ptr: *const Arc<ServiceNetRs>,
    hd: ConnId,
    input_data: *const u8,
    input_len: usize,
) {
    let srv_net = unsafe { &*srv_net_ptr };

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
                    // trigger pkt_fn
                    hd.run_pkt_fn(&conn.srv, srv_net, pkt);
                }

                //
                consumed = input_len - remain;

                if 0 == remain {
                    break;
                }
            }
        } else {
            //
            log::error!("[on_message_cb][hd={:?}] not found!!!", hd);
        }
    }
}

extern "C" fn on_close_cb(srv_net_ptr: *const Arc<ServiceNetRs>, hd: ConnId) {
    let srv_net = unsafe { &*srv_net_ptr };

    //
    {
        let conn_table = srv_net.conn_table.read();
        if let Some(conn) = conn_table.get(&hd) {
            // trigger close_fn
            hd.run_close_fn(&conn.srv, srv_net);
        }
    }
}
