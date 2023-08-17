use parking_lot::RwLock;
use std::sync::Arc;

use crate::service_net::{ConnId, TcpServer, TcpServerHandler};
use crate::ServiceNetRs;

use message_io::network::{NetEvent, NetworkController, ResourceId, Transport};
use message_io::node::{split, NodeHandler, NodeListener, NodeTask};

pub struct MessageIoServer {
    pub node_handler: NodeHandler<()>,
    pub node_listener_opt: Option<NodeListener<()>>, // NOTICE: use option as temp storage
    pub node_task_opt: Option<NodeTask>,

    tcp_server_handler: TcpServerHandler,
    tcp_server_id: usize,
}

impl MessageIoServer {
    ///
    pub fn new(tcp_server_handler: TcpServerHandler, tcp_server_id: usize) -> MessageIoServer {
        // Create a node, the main message-io entity. It is divided in 2 parts:
        // The 'handler', used to make actions (connect, send messages, signals, stop the node...)
        // The 'listener', used to read events from the network or signals.
        let (node_handler, node_listener) = split::<()>();

        MessageIoServer {
            node_handler,
            node_listener_opt: Some(node_listener),
            node_task_opt: None,

            tcp_server_handler,
            tcp_server_id,
        }
    }

    ///
    pub fn listen(
        &mut self,
        addr: &str,
        srv_net: &'static ServiceNetRs,
    ) -> std::io::Result<(ResourceId, std::net::SocketAddr)> {
        let ret = self.node_handler.network().listen(Transport::Tcp, addr);

        log::info!("server running at {}", addr);
        let tcp_server = self.tcp_server_id as *const TcpServer;
        (self.tcp_server_handler.on_listen)(srv_net as *const ServiceNetRs, tcp_server);
        ret
    }

    ///
    pub fn stop(&mut self) {
        self.node_handler.stop();

        let node_task = self.node_task_opt.take().unwrap();
        drop(node_task);
    }
}

/// 启动 message io server，因为需要跨线程传递自身引用，所以不能使用 self method，需要采用关联函数避免重复借用
pub fn start_message_io_server_async(
    server_opt: &Arc<RwLock<Option<MessageIoServer>>>,
    srv_net: &'static ServiceNetRs,
) {
    let node_task = create_message_io_server_node_task(server_opt, srv_net);

    // mount node task opt
    {
        let mut server_opt_mut = server_opt.write();
        (*server_opt_mut).as_mut().unwrap().node_task_opt = Some(node_task);
    }
}

fn create_message_io_server_node_task(
    server_opt1: &Arc<RwLock<Option<MessageIoServer>>>,
    srv_net: &'static ServiceNetRs,
) -> NodeTask {
    //
    let mut server_opt_mut = server_opt1.write();
    let node_listener = server_opt_mut
        .as_mut()
        .unwrap()
        .node_listener_opt
        .take()
        .unwrap();

    // read incoming network events.
    let server_opt2 = server_opt1.clone();
    let node_task = node_listener.for_each_async(move |event| {
        let mut server_opt_mut = server_opt2.write();
        let server_mut = server_opt_mut.as_mut().unwrap();

        let srv_net = srv_net as *const ServiceNetRs;

        let tcp_server_handler = &server_mut.tcp_server_handler;
        let tcp_server = server_mut.tcp_server_id as *const TcpServer;
        let node_network = server_mut.node_handler.network() as *const NetworkController;

        match event.network() {
            NetEvent::Connected(_, _) => {
                unreachable!();
            } // There is no connect() calls.

            NetEvent::Accepted(endpoint, id) => {
                //
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                log::info!(
                    "[hd={:?}] {} endpoint {} accepted, raw_id={}",
                    hd,
                    id,
                    endpoint,
                    raw_id
                );

                //
                (tcp_server_handler.on_accept)(
                    srv_net,
                    tcp_server,
                    node_network,
                    ConnId::from(raw_id),
                    endpoint.addr().into(),
                );
            } // All endpoint accepted

            NetEvent::Message(endpoint, input_data) => {
                //
                (tcp_server_handler.on_message)(
                    srv_net,
                    tcp_server,
                    ConnId::from(endpoint.resource_id().raw()),
                    input_data.as_ptr(),
                    input_data.len(),
                );
            }

            NetEvent::Disconnected(endpoint) => {
                //
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                log::info!("[hd={:?}] endpoint {} disconnected", hd, endpoint);

                //
                (tcp_server_handler.on_close)(srv_net, tcp_server, hd);
            }
        }
    });

    //
    node_task
}
