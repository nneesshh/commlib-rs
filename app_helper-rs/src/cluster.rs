use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};

use atomic::{Atomic, Ordering};
use hashbrown::HashMap;
use net_packet::CmdId;
use prost::Message;

use commlib::{ConnId, NetProxy, NodeId, PacketType, TcpConn};

use super::proto;

static WAITER_NEXT_ID: Atomic<u64> = Atomic::new(1_u64);

///
pub type ReturnHander = Box<dyn Fn(&mut NetProxy, &TcpConn, CmdId, &[u8])>;

///节点数据
pub struct NodeData {
    pub nid: NodeId,                //节点ID
    conn_opt: Option<Arc<TcpConn>>, //tcp 连接信息
}
struct Waiter {
    nodes: Vec<NodeId>,
    cb: Box<dyn Fn()>,
}

///
pub struct Cluster {
    // net proxy
    pub net_proxy: NetProxy,

    // my NodeData
    pub my_node: NodeData,

    // nid,NodeData
    pub nodes: HashMap<NodeId, Rc<NodeData>>,

    // node ready callback
    node_ready_cb: Box<dyn Fn(&mut NetProxy, &NodeData)>,

    // handshake callback
    handshake_cb: Box<dyn Fn(&mut NetProxy, &proto::InnerNodeInfo)>,

    // wait node list
    wait_nodes: Vec<Waiter>,

    // waiting packet
    waiting_handlers: HashMap<NodeId, ReturnHander>,
}

impl Cluster {
    ///
    pub fn new(packet_type: PacketType) -> Rc<RefCell<Self>> {
        let cluster = Self {
            net_proxy: NetProxy::new(packet_type),
            my_node: NodeData {
                nid: 0,
                conn_opt: None,
            },
            nodes: HashMap::default(),
            node_ready_cb: Box::new(|_1, _2| {}),
            handshake_cb: Box::new(|_1, _2| {}),
            wait_nodes: Vec::default(),
            waiting_handlers: HashMap::default(),
        };
        let cluster = Rc::new(RefCell::new(cluster));
        regitser_packet_handler(&cluster);
        cluster
    }

    /// 握手
    pub fn handle_node_handshake(
        &mut self,
        proxy: &mut NetProxy,
        conn: &TcpConn,
        cmd: CmdId,
        data: &[u8],
    ) {
        let result = proto::InnerNodeInfo::decode(data);
        match result {
            Ok(msg) => {
                self.update_node_info(conn, &msg);

                let my = &self.my_node;
                let ntf = proto::InnerNodeInfo {
                    nid: my.nid,
                    r#type: my.nid as i32,
                    sids: Vec::default(),
                    kv: Vec::default(),
                    maxnum: 0,
                };

                (self.handshake_cb)(proxy, &ntf);

                self.net_proxy.send_proto(
                    conn,
                    proto::InnerReservedCmd::IrcNodeInfoNtf as CmdId,
                    &ntf,
                );
            }
            Err(err) => {
                log::error!(
                    "[Cluster::handle_node_handshake()] InnerNodeInfo decode failed err:{}, cmd={cmd:?}",
                    err
                );
            }
        }
    }

    /// 握手成功通知
    pub fn handle_node_info_notify(
        &mut self,
        _proxy: &mut NetProxy,
        conn: &TcpConn,
        cmd: CmdId,
        data: &[u8],
    ) {
        let result = proto::InnerNodeInfo::decode(data);
        match result {
            Ok(msg) => {
                self.update_node_info(conn, &msg);
            }
            Err(err) => {
                log::error!(
                    "[Cluster::handle_node_info_notify()] InnerNodeInfo decode failed err:{}, cmd={cmd:?}",
                    err
                );
            }
        }
    }

    /// 回包
    pub fn handle_rpc_return(
        &mut self,
        proxy: &mut NetProxy,
        conn: &TcpConn,
        cmd: CmdId,
        data: &[u8],
    ) {
        let result = proto::InnerRpcReturn::decode(data);
        match result {
            Ok(msg) => {
                let wait_handler = &mut self.waiting_handlers;
                if let Some(cb) = wait_handler.get(&msg.rpc_id) {
                    cb(proxy, conn, cmd, data);
                    wait_handler.remove(&msg.rpc_id);
                } else {
                    log::error!(
                        "[Cluster::handle_rpc_return()] InnerRpcReturn call back err not register rpcid:{}",
                        msg.rpc_id
                    );
                }
            }
            Err(err) => {
                log::error!(
                    "[Cluster::handle_rpc_return()] InnerRpcReturn decode failed err:{}",
                    err
                );
            }
        }
    }

    /// 设置成功连接回调
    pub fn wait_connected<F>(&mut self, nodes: Vec<NodeId>, f: F)
    where
        F: Fn() + 'static,
    {
        let wait = Waiter {
            nodes,
            cb: Box::new(f),
        };

        self.wait_nodes.push(wait);
    }

    /// 更新等待的节点列表
    pub fn check_waiter(&mut self) {
        self.wait_nodes.retain(|waiter| {
            for nid in &waiter.nodes {
                let ret = self.nodes.get(nid);
                if ret.is_some() && self.my_node.nid != *nid {
                    (waiter.cb)();
                    return false; // remove
                }
            }

            true
        });
    }

    /// 连接成功后，发送节点信息
    pub fn on_connect(&mut self, conn: &Arc<TcpConn>) {
        let mut req = proto::InnerNodeInfo {
            nid: 0,
            r#type: 0,
            sids: Vec::default(),
            kv: Vec::default(),
            maxnum: 0i32,
        };

        let my = &self.my_node;
        req.nid = my.nid;
        req.r#type = my.nid as i32;

        //
        self.net_proxy.send_proto(
            &*conn,
            proto::InnerReservedCmd::IrcNodeHandshake as CmdId,
            &req,
        );
    }

    /// 连接断开
    pub fn on_close(&mut self, hd: ConnId) {
        self.nodes.retain(|_, node| {
            if let Some(conn) = &node.conn_opt {
                if conn.hd == hd {
                    return false; // remove
                }
            }

            true
        });
    }

    /// set node ready callback
    pub fn set_node_ready_cb<F>(&mut self, f: F)
    where
        F: Fn(&mut NetProxy, &NodeData) + 'static,
    {
        self.node_ready_cb = Box::new(f);
    }

    /// 设置自己节点信息
    pub fn set_my_node_info(&mut self, nid: NodeId) {
        self.my_node = NodeData {
            nid,
            conn_opt: None,
        };
    }

    /// 更新节点数据
    pub fn update_node_info(&mut self, conn: &TcpConn, info: &proto::InnerNodeInfo) {
        //
        let hd = conn.hd;
        let nid = info.nid;
        let arced_conn = self.net_proxy.get_conn(hd);

        //
        let node_opt = self.nodes.remove(&nid);
        match node_opt {
            Some(node) => {
                if let Some(node_conn) = &node.conn_opt {
                    if node_conn.hd != hd {
                        log::info!(
                            "[Cluster::update_node_info()][hd={}] info.hd={}",
                            hd,
                            node_conn.hd,
                        );
                    }
                } else {
                    log::info!(
                        "[Cluster::update_node_info() [hd={}] my_node: {}",
                        hd,
                        self.my_node.nid
                    );
                }
            }
            None => {
                log::info!(
                    "[Cluster::update_node_info()] insert node [hd={}] my_node: {}",
                    hd,
                    self.my_node.nid
                );
            }
        }

        //
        let node = NodeData {
            nid,
            conn_opt: Some(arced_conn.clone()),
        };
        (self.node_ready_cb)(&mut self.net_proxy, &node);
        self.nodes.insert(
            nid,
            Rc::new(NodeData {
                nid,
                conn_opt: Some(arced_conn.clone()),
            }),
        );

        // 有新的节点数据上报过来，更新一下等待的节点列表
        self.check_waiter();
    }

    /// 消息发送
    #[inline(always)]
    pub fn send(&mut self, conn: &TcpConn, cmd: CmdId, msg: &impl prost::Message) {
        self.net_proxy.send_proto(conn, cmd, msg);
    }

    /// 发送到指定节点
    #[inline(always)]
    pub fn send_to_server(&mut self, nid: NodeId, cmd: CmdId, msg: &impl prost::Message) {
        let node_opt = self.nodes.get(&nid);
        if let Some(node) = node_opt {
            if let Some(conn) = &node.conn_opt {
                self.net_proxy.send_proto(&conn, cmd, msg);
            } else {
                log::error!(
                    "[Cluster::send_to_server()] cmd:{} nid:{} send failed because conn is none!!!",
                    cmd,
                    nid
                );
            }
        } else {
            log::error!(
                "[Cluster::send_to_server()] cmd:{} nid:{} send failed because node not exists!!!",
                cmd,
                nid
            );
        }
    }

    /// 发送到所有节点
    #[inline(always)]
    pub fn send_to_all_nodes(&mut self, cmd: CmdId, msg: &impl prost::Message) {
        for (_, node) in &self.nodes {
            if let Some(conn) = &node.conn_opt {
                self.net_proxy.send_proto(&conn, cmd, msg);
            } else {
                log::error!(
                    "[Cluster::send_to_all_nodes()] cmd:{} nid:{} send failed because conn is none!!!",
                    cmd,
                    node.nid
                );
            }
        }
    }

    /// 发送到指定节点
    #[inline(always)]
    pub fn rpc_to_server(&mut self, nid: NodeId, cmd: CmdId, msg: &impl prost::Message) {
        self.send_to_server(nid, cmd, msg);
    }

    /// 发送到指定节点 等待回包
    #[inline(always)]
    pub fn call_rpc_to_server<F>(
        &mut self,
        nid: NodeId,
        cmd: CmdId,
        msg: &impl prost::Message,
        f: F,
    ) where
        F: Fn(&mut NetProxy, &TcpConn, CmdId, &[u8]) + 'static,
    {
        let id = WAITER_NEXT_ID.fetch_add(1, Ordering::Relaxed);
        self.waiting_handlers.insert(id, Box::new(f));
        self.send_to_server(nid, cmd, msg);
    }
}

/// use thread local unsafe cell -- mut
#[macro_export]
macro_rules! cluster_register_packet_handler {
    ($source:ident, $cmd:path, $member_fn:ident) => {
        {
            let clone1 = $source.clone();
            let mut s = $source.borrow_mut();
            s.net_proxy.set_packet_handler(
                $cmd as CmdId,
                move |proxy, conn, cmd, data| {
                    let ret = clone1.try_borrow_mut();
                    match ret {
                        Ok(mut s) => {
                            paste::paste! {
                                s.[< $member_fn >](proxy, conn, cmd, data);
                            }
                        }
                        Err(err) => {
                            log::error!("source try_borrow_mut error: {:?}!!! cmd={cmd:?}!!!", err);
                        }
                    }
                },
            );
        }
    };
}

/// 注册消息监听
pub fn regitser_packet_handler(cluster: &Rc<RefCell<Cluster>>) {
    //
    cluster_register_packet_handler!(cluster, proto::InnerReservedCmd::IrcNodeHandshake, handle_node_handshake);
    cluster_register_packet_handler!(cluster, proto::InnerReservedCmd::IrcNodeInfoNtf, handle_node_info_notify);
    cluster_register_packet_handler!(cluster, proto::InnerReservedCmd::IrcRpcReturn, handle_rpc_return);
 }
