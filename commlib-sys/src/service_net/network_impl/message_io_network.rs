use parking_lot::RwLock;
use std::sync::Arc;

use crate::{ConnId, ServiceNetRs, TcpHandler, TcpListenerId, TcpServer};

use message_io::network::{NetEvent, Transport};
use message_io::node::{split, NodeHandler, NodeListener, NodeTask};

/// message io
pub struct MessageIoNetwork {
    pub node_handler: NodeHandler<()>,
    pub node_listener_opt: Option<NodeListener<()>>, // NOTICE: use option as temp storage
    pub node_task_opt: Option<NodeTask>,

    tcp_handler: TcpHandler,
}

impl MessageIoNetwork {
    ///
    pub fn new() -> MessageIoNetwork {
        // Create a node, the main message-io entity. It is divided in 2 parts:
        // The 'handler', used to make actions (connect, send messages, signals, stop the node...)
        // The 'listener', used to read events from the network or signals.
        let (node_handler, node_listener) = split::<()>();

        MessageIoNetwork {
            node_handler,
            node_listener_opt: Some(node_listener),
            node_task_opt: None,

            tcp_handler: TcpHandler::new(),
        }
    }

    ///
    pub fn listen(
        &mut self,
        addr: &str,
        tcp_server: &mut TcpServer,
        srv_net: &Arc<ServiceNetRs>,
    ) -> bool {
        let network = self.node_handler.network();
        let ret = network.listen(Transport::Tcp, addr);

        log::info!("network listening at {}", addr);

        let srv_net_ptr = &(*srv_net) as *const Arc<ServiceNetRs>;
        let tcp_server_ptr = tcp_server as *const TcpServer;

        //
        match ret {
            Ok((id, sock_addr)) => {
                let listener_id = TcpListenerId::from(id.raw());
                (self.tcp_handler.on_listen)(
                    srv_net_ptr,
                    tcp_server_ptr,
                    listener_id,
                    sock_addr.into(),
                );
                true
            }

            Err(error) => {
                log::error!("network listening at {} failed!!! err {:?}", addr, error);
                false
            }
        }
    }

    ///
    pub fn connect(&mut self, raddr: &str) {
        log::info!("start connect to raddr: {}", raddr);
        let network = self.node_handler.network();
        match network.connect_sync(Transport::Tcp, raddr) {
            Ok((endpoint, _)) => {
                //
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                log::info!("[hd={:?}] client connected, raddr: {}", hd, raddr);

                //
                network.send(endpoint, &[42]);
            }
            Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => {
                log::info!("Could not connect to raddr: {}!!!", raddr);
            }
            Err(err) => {
                log::info!("An OS error creating the socket, raddr: {}!!!", raddr);
            }
        }
    }

    ///
    pub fn stop(&mut self) {
        self.node_handler.stop();

        let node_task = self.node_task_opt.take().unwrap();
        drop(node_task);
    }
}

/// 启动 message io server，因为需要跨线程传递自身引用，所以不能使用 self method，需要采用关联函数避免重复借用
pub fn start_message_io_network_async(
    network: &Arc<RwLock<MessageIoNetwork>>,
    srv_net: &Arc<ServiceNetRs>,
) {
    let node_task = create_message_io_network_node_task(network, srv_net);

    // mount node task opt
    {
        let mut network_mut = network.write();
        (*network_mut).node_task_opt = Some(node_task);
    }
}

fn create_message_io_network_node_task(
    network: &Arc<RwLock<MessageIoNetwork>>,
    srv_net: &Arc<ServiceNetRs>,
) -> NodeTask {
    //
    let node_listener;
    {
        let mut network_mut = network.write();
        node_listener = network_mut.node_listener_opt.take().unwrap();
    }

    let on_accept;
    let on_message;
    let on_close;
    let node_handler;
    {
        let network = network.read();

        on_accept = network.tcp_handler.on_accept;
        on_message = network.tcp_handler.on_message;
        on_close = network.tcp_handler.on_close;

        node_handler = network.node_handler.clone();
    }

    //
    let srv_net = srv_net.clone();

    // read incoming network events.
    let node_task = node_listener.for_each_async(move |event| {
        //
        let netctrl_ptr = &node_handler as *const NodeHandler<()>;
        let srv_net_ptr = &srv_net as *const Arc<ServiceNetRs>;

        //
        match event.network() {
            NetEvent::Connected(_, _) => {
                unreachable!();
            } // There is no connect() calls.

            NetEvent::Accepted(endpoint, id) => {
                //
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                let listener_id = TcpListenerId::from(id.raw());
                log::info!(
                    "[hd={:?}] {} endpoint {} accepted, listener_id={:?}",
                    hd,
                    raw_id,
                    endpoint,
                    listener_id
                );

                //
                (on_accept)(
                    srv_net_ptr,
                    netctrl_ptr,
                    listener_id,
                    hd,
                    endpoint.addr().into(),
                );
            } // All endpoint accepted

            NetEvent::Message(endpoint, input_data) => {
                //
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);

                //
                (on_message)(srv_net_ptr, hd, input_data.as_ptr(), input_data.len());
            }

            NetEvent::Disconnected(endpoint) => {
                //
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                log::info!("[hd={:?}] endpoint {} disconnected", hd, endpoint);

                //
                (on_close)(srv_net_ptr, hd);
            }
        }
    });

    //
    node_task
}
