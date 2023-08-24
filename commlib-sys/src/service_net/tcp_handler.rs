use std::sync::Arc;

use crate::service_net::PacketType;
use message_io::network::{Endpoint, ResourceId};
use message_io::node::NodeHandler;

use super::{ConnId, OsSocketAddr, ServiceNetRs, TcpClient, TcpConn, TcpListenerId, TcpServer};

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
        let mut conn = TcpConn::new(PacketType::Server, hd, endpoint, netctrl, &srv);
        listener_id.bind_callbacks(&mut conn, srv_net);

        {
            let mut conn_table_mut = srv_net.conn_table.write();
            (*conn_table_mut).insert(hd, conn);
        }
    }

    // trigger conn_fn
    hd.run_conn_fn(&srv, srv_net);
}

extern "C" fn on_connected_cb(tcp_client_ptr: *const TcpClient, hd: ConnId, os_addr: OsSocketAddr) {
    let tcp_client = unsafe { &mut *(tcp_client_ptr as *mut TcpClient) };

    let srv_net = &tcp_client.srv_net;
    let srv = &tcp_client.srv;

    let id = ResourceId::from(hd.id);
    let sock_addr = os_addr.into_addr().unwrap();
    let endpoint = Endpoint::new(id, sock_addr);

    // update inner hd for TcpClient
    tcp_client.inner_hd = hd;

    // bind new conn to tcp client
    {
        let netctrl = &tcp_client.mi_network.node_handler;
        let conn = TcpConn::new(PacketType::Server, hd, endpoint, netctrl, srv);
    }
}

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
            let mut pos = 0_usize;
            loop {
                let ptr = unsafe { input_data.offset(pos as isize) };
                let len = input_len - pos;
                match conn.handle_read(ptr, len) {
                    Ok((Some(pkt), consumed)) => {
                        // 收到一个 pkt trigger pkt_fn
                        hd.run_pkt_fn(&conn.srv, srv_net, pkt);
                        pos += consumed;
                    }
                    Ok((None, consumed)) => {
                        // pkt 尚不完整,  continue
                        pos += consumed;
                    }
                    Err(err) => {
                        // disconnect
                        log::error!("[on_message_cb] handle_read failed!!! error: {}", err);
                        hd.disconnet(srv_net);
                        break;
                    }
                }

                //
                assert!(pos <= input_len);
                if pos == input_len {
                    break;
                }
            }
        } else {
            //
            log::error!("[on_message_cb][hd={}] conn not found!!!", hd);
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
