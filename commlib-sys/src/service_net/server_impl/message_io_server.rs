use crate::service_net::{ConnId, TcpServer, TcpServerHandler};
use crate::ServiceNetRs;
use message_io;
use message_io::network::NetworkController;

pub struct MessageIoServer {
    node_handler: message_io::node::NodeHandler<()>,
    node_listener: Option<message_io::node::NodeListener<()>>, // NOTICE: use option as temp storage

    tcp_server_handler: TcpServerHandler,
    tcp_server_id: usize,
}

impl MessageIoServer {
    ///
    pub fn new(tcp_server_handler: TcpServerHandler, tcp_server_id: usize) -> MessageIoServer {
        // Create a node, the main message-io entity. It is divided in 2 parts:
        // The 'handler', used to make actions (connect, send messages, signals, stop the node...)
        // The 'listener', used to read events from the network or signals.
        let (node_handler, node_listener) = message_io::node::split::<()>();

        MessageIoServer {
            node_handler,
            node_listener: Some(node_listener),

            tcp_server_handler,
            tcp_server_id,
        }
    }

    ///
    pub fn listen(
        &self,
        addr: &str,
        srv_net: &'static ServiceNetRs,
    ) -> std::io::Result<(message_io::network::ResourceId, std::net::SocketAddr)> {
        let ret = self
            .node_handler
            .network()
            .listen(message_io::network::Transport::Tcp, addr);

        log::info!("server running at {}", addr);
        let tcp_server = self.tcp_server_id as *const TcpServer;
        (self.tcp_server_handler.on_listen)(srv_net as *const ServiceNetRs, tcp_server);
        ret
    }

    ///
    pub fn run(&mut self, srv_net: &'static ServiceNetRs) {
        let node_listener = self.node_listener.take().unwrap();

        // Read incoming network events.
        node_listener.for_each(move |event| match event.network() {
            message_io::network::NetEvent::Connected(_, _) => unreachable!(), // There is no connect() calls.
            message_io::network::NetEvent::Accepted(endpoint, _id) => {
                let srv_net = srv_net as *const ServiceNetRs;
                let tcp_server = self.tcp_server_id as *const TcpServer;
                let network = self.node_handler.network() as *const NetworkController;

                let raw_id = endpoint.resource_id().raw();

                //
                (self.tcp_server_handler.on_accept)(
                    srv_net,
                    tcp_server,
                    network,
                    ConnId::from(raw_id),
                    endpoint.addr().into(),
                );
            } // All endpoint accepted
            message_io::network::NetEvent::Message(endpoint, input_data) => {
                let srv_net = srv_net as *const ServiceNetRs;
                let tcp_server = self.tcp_server_id as *const TcpServer;

                //
                (self.tcp_server_handler.on_message)(
                    srv_net,
                    tcp_server,
                    ConnId::from(endpoint.resource_id().raw()),
                    input_data.as_ptr(),
                    input_data.len(),
                );
            }
            message_io::network::NetEvent::Disconnected(endpoint) => {
                let srv_net = srv_net as *const ServiceNetRs;
                let tcp_server = self.tcp_server_id as *const TcpServer;

                //
                (self.tcp_server_handler.on_close)(
                    srv_net,
                    tcp_server,
                    ConnId::from(endpoint.resource_id().raw()),
                );
            }
        });
    }
}
