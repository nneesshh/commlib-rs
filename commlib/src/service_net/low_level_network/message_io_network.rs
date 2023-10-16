use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;

use message_io::network::{Endpoint, NetEvent, ResourceId, Transport};
use message_io::node::{split, NodeHandler, NodeListener, NodeTask};

use crate::service_net::connector::insert_connector;
use crate::{ConnId, Connector, ServiceNetRs, TcpHandler, TcpListenerId, TcpServer};

/// message io
pub struct MessageIoNetwork {
    pub node_handler: NodeHandler<()>,
    pub node_listener_opt: RwLock<Option<NodeListener<()>>>, // NOTICE: use option as temp storage
    pub node_task_opt: RwLock<Option<NodeTask>>,

    tcp_handler: TcpHandler,
}

impl MessageIoNetwork {
    ///
    pub fn new() -> Self {
        // Create a node, the main message-io entity. It is divided in 2 parts:
        // The 'handler', used to make actions (connect, send messages, signals, stop the node...)
        // The 'listener', used to read events from the network or signals.
        let (node_handler, node_listener) = split::<()>();

        Self {
            node_handler,
            node_listener_opt: RwLock::new(Some(node_listener)),
            node_task_opt: RwLock::new(None),

            tcp_handler: TcpHandler::new(),
        }
    }

    ///
    pub fn listen(&self, tcp_server: &mut TcpServer) -> bool {
        //
        let addr = tcp_server.addr.to_owned();
        let ret = self
            .node_handler
            .network()
            .listen(Transport::Tcp, addr.as_str());

        let tcp_server_ptr = tcp_server as *mut TcpServer;

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

    /// connect with connector
    pub fn connect_with_connector(
        &self,
        connector: &Arc<Connector>,
        sock_addr: SocketAddr,
        srv_net: &ServiceNetRs,
    ) {
        //
        match self
            .node_handler
            .network()
            .connect(Transport::Tcp, sock_addr)
        {
            //
            Ok((endpoint, _sock_addr)) => {
                // async, add connector only, see "NetEvent::Connected" for real callback
                let raw_id = endpoint.resource_id().raw();
                let hd = ConnId::from(raw_id);
                insert_connector(srv_net, hd, connector);
            }
            Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => {
                log::error!(
                    "Could not connect to sock_addr: {}!!! error: {}",
                    sock_addr,
                    err
                );
                (connector.ready_cb)(Err("ConnectionRefused".to_owned()));
            }
            Err(err) => {
                log::error!(
                    "Could not connect to sock_addr: {}!!! error: {}",
                    sock_addr,
                    err
                );
                (connector.ready_cb)(Err(err.to_string()));
            }
        };
    }

    ///
    pub fn stop(&self) {
        self.node_handler.stop();

        {
            let mut node_task_mut = self.node_task_opt.write();
            let node_task = node_task_mut.take().unwrap();
            drop(node_task);
        }
    }

    ///
    #[inline(always)]
    pub fn close(&self, hd: ConnId) {
        let rid = ResourceId::from(hd.id);
        self.node_handler.network().remove(rid);
    }

    ///
    #[inline(always)]
    pub fn send(&self, hd: ConnId, sock_addr: SocketAddr, data: &[u8]) {
        let rid = ResourceId::from(hd.id);
        let endpoint = Endpoint::new(rid, sock_addr);
        self.node_handler.network().send(endpoint, data);
    }

    /// 异步启动 message io network
    pub fn start_network_async(&self, srv_net: &Arc<ServiceNetRs>) {
        let node_task = self.create_node_task(srv_net);

        // mount node task opt
        {
            let mut node_task_opt_mut = self.node_task_opt.write();
            (*node_task_opt_mut) = Some(node_task);
        }
    }

    fn create_node_task(&self, srv_net: &Arc<ServiceNetRs>) -> NodeTask {
        //
        let node_listener;
        {
            let mut node_listener_opt_mut = self.node_listener_opt.write();
            node_listener = node_listener_opt_mut.take().unwrap();
        }

        //
        let on_connect_ok = self.tcp_handler.on_connect_ok;
        let on_connect_err = self.tcp_handler.on_connect_err;
        let on_accept = self.tcp_handler.on_accept;
        let on_input = self.tcp_handler.on_input;
        let on_close = self.tcp_handler.on_close;

        //
        let srv_net = srv_net.clone();

        // read incoming network events.
        let node_task = node_listener.for_each_async(move |event| {
            //
            let srv_net_ptr = &srv_net as *const Arc<ServiceNetRs>;

            //
            match event.network() {
                NetEvent::Connected(endpoint, handshake) => {
                    //
                    let raw_id = endpoint.resource_id().raw();
                    let os_addr = endpoint.addr().into();
                    let hd = ConnId::from(raw_id);

                    log::info!(
                        "[hd={}] {} endpoint {} handshake={}.",
                        hd,
                        raw_id,
                        endpoint,
                        handshake
                    );

                    if handshake {
                        // connect ok
                        (on_connect_ok)(srv_net_ptr, hd, os_addr);
                    } else {
                        // connect err
                        (on_connect_err)(srv_net_ptr, hd);
                    }
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
                    (on_accept)(srv_net_ptr, listener_id, hd, endpoint.addr().into());
                } // NetEvent::Accepted

                NetEvent::Message(endpoint, input_data) => {
                    //
                    let raw_id = endpoint.resource_id().raw();
                    let hd = ConnId::from(raw_id);

                    //
                    (on_input)(srv_net_ptr, hd, input_data.as_ptr(), input_data.len());
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
}
