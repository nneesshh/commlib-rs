use parking_lot::RwLock;
use std::sync::Arc;

use crate::{ConnId, ServiceNetRs, TcpClient, TcpHandler, TcpListenerId, TcpServer};

use crate::service_net::TcpConn;
use message_io::network::{NetEvent, Transport};
use message_io::node::{split, NodeHandler, NodeListener, NodeTask};

/// message io
pub struct MessageIoNetwork {
    pub node_handler: NodeHandler<()>,
    pub node_listener_opt: RwLock<Option<NodeListener<()>>>, // NOTICE: use option as temp storage
    pub node_task_opt: RwLock<Option<NodeTask>>,

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
            node_listener_opt: RwLock::new(Some(node_listener)),
            node_task_opt: RwLock::new(None),

            tcp_handler: TcpHandler::new(),
        }
    }

    ///
    pub fn listen(&self, tcp_server: &mut TcpServer) -> bool {
        //
        let addr = tcp_server.addr.as_str();
        let ret = self.node_handler.network().listen(Transport::Tcp, addr);

        log::info!("network listening at {}", addr);

        let tcp_server_ptr = tcp_server as *const TcpServer;

        //
        match ret {
            Ok((id, sock_addr)) => {
                let listener_id = TcpListenerId::from(id.raw());
                (self.tcp_handler.on_listen)(tcp_server_ptr, listener_id, sock_addr.into());
                true
            }

            Err(err) => {
                log::error!("network listening at {} failed!!! error {:?}", addr, err);
                false
            }
        }
    }

    ///
    pub fn connect(&self, tcp_client: &mut TcpClient) -> Result<ConnId, String> {
        let srv_net_ptr = &tcp_client.srv_net as *const Arc<ServiceNetRs>;
        let netctrl_ptr = &self.node_handler as *const NodeHandler<()>;
        let tcp_client_ptr = tcp_client as *mut TcpClient;

        let raddr = tcp_client.raddr.as_str();
        log::info!("start connect to raddr: {}", raddr);

        //
        match self.node_handler.network().connect(Transport::Tcp, raddr) {
            Ok((endpoint, sock_addr)) => {
                //
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                log::info!(
                    "[hd={}] client connected, raddr: {} sock_addr: {}",
                    hd,
                    raddr,
                    sock_addr
                );

                // call on_connected directly
                let on_connected = self.tcp_handler.on_connected;
                on_connected(tcp_client_ptr, hd, sock_addr.into());

                //
                Ok(hd)
            }
            Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => {
                log::error!("Could not connect to raddr: {}!!! error: {}", raddr, err);
                Err("ConnectionRefused".to_owned())
            }
            Err(err) => {
                log::error!("Could not connect to raddr: {}!!! error: {}", raddr, err);
                Err(err.to_string())
            }
        }
    }

    ///
    pub fn stop(&self) {
        self.node_handler.stop();

        {
            let mut node_task_mut = self.node_task_opt.write();
            let node_task = (*node_task_mut).take().unwrap();
            drop(node_task);
        }
    }
}

/// 启动 message io server，因为需要跨线程传递自身引用，所以不能使用 self method，需要采用关联函数避免重复借用
pub fn start_message_io_network_async(
    mi_network: &Arc<MessageIoNetwork>,
    srv_net: &Arc<ServiceNetRs>,
) {
    let node_task = create_message_io_network_node_task(mi_network, srv_net);

    // mount node task opt
    {
        let mut node_task_opt_mut = mi_network.node_task_opt.write();
        (*node_task_opt_mut) = Some(node_task);
    }
}

fn create_message_io_network_node_task(
    mi_network: &Arc<MessageIoNetwork>,
    srv_net: &Arc<ServiceNetRs>,
) -> NodeTask {
    //
    let node_listener;
    {
        let mut node_listener_opt_mut = mi_network.node_listener_opt.write();
        node_listener = (*node_listener_opt_mut).take().unwrap();
    }

    //
    let on_accept = mi_network.tcp_handler.on_accept;
    let on_message = mi_network.tcp_handler.on_message;
    let on_close = mi_network.tcp_handler.on_close;

    let node_handler = mi_network.node_handler.clone();

    //
    let srv_net = srv_net.clone();

    // read incoming network events.
    let node_task = node_listener.for_each_async(move |event| {
        //
        let srv_net_ptr = &srv_net as *const Arc<ServiceNetRs>;
        let netctrl_ptr = &node_handler as *const NodeHandler<()>;

        //
        match event.network() {
            NetEvent::Connected(endpoint, handshake) => {
                // just log
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                log::info!(
                    "[hd={}] {} endpoint {} connected, handshake={}.",
                    hd,
                    raw_id,
                    endpoint,
                    handshake
                );
            }

            NetEvent::Accepted(endpoint, id) => {
                //
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                let listener_id = TcpListenerId::from(id.raw());
                log::info!(
                    "[hd={}] {} endpoint {} accepted, listener_id={}",
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
            } // NetEvent::Accepted

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
                log::info!("[hd={}] endpoint {} disconnected", hd, endpoint);

                //
                (on_close)(srv_net_ptr, hd);
            }
        }
    });

    //
    node_task
}
