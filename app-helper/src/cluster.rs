use std::collections::LinkedList;
use std::rc::Rc;
use std::sync::Arc;

use commlib::{ConnId, NodeId, TcpConn};

///
pub struct NodeInfo {
    nid: NodeId,  // 节点 id
    ntype: usize, // 节点类型
    conn: Arc<TcpConn>,

    values: hashbrown::HashMap<String, String>,
}

///
pub struct Waiter {
    nodes: LinkedList<NodeId>,
    cb: Box<dyn Fn() + Send + Sync>,
}

///
pub struct Cluster {
    my_: NodeInfo,

    nodes_: hashbrown::HashMap<NodeId, Rc<NodeInfo>>, // nid->node
    hd_nodes_: hashbrown::HashMap<ConnId, Rc<NodeInfo>>, // hd->node
    type_nodes_: hashbrown::HashMap<ConnId, LinkedList<Rc<NodeInfo>>>, // type -> node list

    waiters_: LinkedList<Waiter>,
    node_listen_cb: Box<dyn Fn(Rc<NodeInfo>) + Send + Sync>,
    node_select_cd: Box<dyn Fn(Rc<NodeInfo>) + Send + Sync>,
}

impl Cluster {
    ///
    pub fn wait_connected<F>(nodes: LinkedList<NodeId>, cb: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
    }

    ///
    pub fn send_to_world(&self, data: &[u8]) {
        //
    }
}
