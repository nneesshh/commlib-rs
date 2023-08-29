use std::sync::Arc;

use message_io::network::{Endpoint, ResourceId};
use message_io::node::NodeHandler;

use super::{packet_reader::PacketResult, PacketType};
use super::{ConnId, OsSocketAddr, ServiceNetRs, TcpClient, TcpListenerId, TcpServer};

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

    // trigger listen_fn
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

    // insert new conn
    listener_id.make_new_conn(PacketType::Server, hd, endpoint, netctrl, srv_net);

    //
    let conn_opt = srv_net.get_conn(hd);
    if let Some(conn) = conn_opt {
        // trigger conn_fn
        conn.run_conn_fn();
    }
}

extern "C" fn on_connected_cb(tcp_client_ptr: *const TcpClient, hd: ConnId, os_addr: OsSocketAddr) {
    let tcp_client = unsafe { &mut *(tcp_client_ptr as *mut TcpClient) };

    let id = ResourceId::from(hd.id);
    let sock_addr = os_addr.into_addr().unwrap();
    let endpoint = Endpoint::new(id, sock_addr);

    // update inner hd for TcpClient
    tcp_client.inner_hd = hd;

    // bind new conn to tcp client
    {
        let netctrl = &tcp_client.mi_network.node_handler;
        tcp_client.make_new_conn(PacketType::Server, hd, endpoint, netctrl);
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
        let conn_opt = srv_net.get_conn(hd);
        if let Some(conn) = conn_opt {
            // conn 循环处理 input
            let mut pos = 0_usize;
            loop {
                let ptr = unsafe { input_data.offset(pos as isize) };
                let len = input_len - pos;
                match conn.handle_read(ptr, len) {
                    PacketResult::Ready((pkt, consumed)) => {
                        // 收到一个 pkt trigger pkt_fn
                        conn.run_pkt_fn(pkt);
                        pos += consumed;
                    }
                    PacketResult::Suspend(consumed) => {
                        // pkt 尚不完整,  continue
                        pos += consumed;
                    }
                    PacketResult::Abort(err) => {
                        // disconnect
                        log::error!("[on_message_cb] handle_read failed!!! error: {}", err);
                        hd.close(srv_net);
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
    let conn_opt = srv_net.get_conn(hd);
    if let Some(conn) = conn_opt {
        // remove conn always
        srv_net.remove_conn(hd);

        // trigger close_fn
        conn.run_close_fn();
    }
}
