use parking_lot::RwLock;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use message_io::network::{Endpoint, Multiplexor, NetEvent, ResourceId, Transport};
use message_io::node::{self, NodeHandler, NodeTask};

use net_packet::NetPacketGuard;

use crate::service_net::connector::{insert_connector, Connector};
use crate::service_net::listener::{insert_listener, Listener};
use crate::service_net::tcp_handler::InputBuffer;
use crate::service_net::{ListenerId, TcpHandler};
use crate::{ConnId, ServiceNetRs, ServiceRs};

/// message io
pub struct MessageIoNetwork {
    pub mux_opt: RwLock<Option<Multiplexor>>, // NOTICE: use option as temp storage
    pub node_handler: NodeHandler,
    pub node_task_opt: RwLock<Option<NodeTask>>,

    tcp_handler: TcpHandler,
}

impl MessageIoNetwork {
    ///
    pub fn new() -> Self {
        // Create a node, the main message-io entity. It is divided in 2 parts:
        // The 'handler', used to make actions (connect, send messages, signals, stop the node...)
        // The 'listener', used to read events from the network or signals.
        let (mux, node_handler) = node::split();

        Self {
            mux_opt: RwLock::new(Some(mux)),
            node_handler,
            node_task_opt: RwLock::new(None),

            tcp_handler: TcpHandler::new(),
        }
    }

    ///
    pub fn listen_with_tcp(
        &self,
        listener: &Arc<Listener>,
        addr: &str,
        srv_net: &Arc<ServiceNetRs>,
    ) -> bool {
        // listen at tcp addr
        let ret = self.node_handler.listen_sync(Transport::Tcp, addr);
        match ret {
            Ok((id, sock_addr)) => {
                let listener_id = ListenerId::from(id.raw());
                insert_listener(srv_net, listener_id, listener);

                //
                let on_listen = self.tcp_handler.on_listen;
                (on_listen)(srv_net, listener_id, sock_addr.into());
                true
            }

            Err(err) => {
                log::error!("network listening at {} failed!!! error {:?}!!!", addr, err);
                false
            }
        }
    }

    ///
    #[cfg(feature = "ssl")]
    pub fn listen_with_ssl(
        &self,
        listener: &Arc<Listener>,
        addr: &str,
        cert_path: PathBuf,
        pri_key_path: PathBuf,
        srv_net: &Arc<ServiceNetRs>,
    ) -> bool {
        // listen at tcp addr
        let config = ListenConfig {
            bind_device_opt: None,
            keepalive_opt: None,
            cert_path,
            pri_key_path,
        };
        let ret = self
            .node_handler
            .listen_sync_with(config, Transport::Ssl, addr);
        match ret {
            Ok((id, sock_addr)) => {
                let listener_id = ListenerId::from(id.raw());
                insert_listener(srv_net, listener_id, listener);

                //
                let on_listen = self.tcp_handler.on_listen;
                (on_listen)(srv_net, listener_id, sock_addr.into());
                true
            }

            Err(err) => {
                log::error!("network listening at {} failed!!! error {:?}!!!", addr, err);
                false
            }
        }
    }

    ///
    #[cfg(feature = "websocket")]
    pub fn listen_with_ws(
        &self,
        listener: &Arc<Listener>,
        addr: &str,
        srv_net: &Arc<ServiceNetRs>,
    ) -> bool {
        // listen at tcp addr
        let config = ListenConfig::default();
        let ret = self
            .node_handler
            .listen_sync_with(config, Transport::Ws, addr);
        match ret {
            Ok((id, sock_addr)) => {
                let listener_id = ListenerId::from(id.raw());
                insert_listener(srv_net, listener_id, listener);

                //
                let on_listen = self.tcp_handler.on_listen;
                (on_listen)(srv_net, listener_id, sock_addr.into());
                true
            }

            Err(err) => {
                log::error!("network listening at {} failed!!! error {:?}!!!", addr, err);
                false
            }
        }
    }

    /// connect with connector
    pub fn connect_with_connector(
        &self,
        connector: &Arc<Connector>,
        sock_addr: SocketAddr,
        srv_net: &Arc<ServiceNetRs>,
    ) {
        //
        let srv_net2 = srv_net.clone();
        let connector = connector.clone();
        let cb = move |ret: io::Result<(Endpoint, SocketAddr)>| {
            match ret {
                Ok((endpoint, _sock_addr)) => {
                    // async, add connector only, see "NetEvent::Connected" for real callback
                    let raw_id = endpoint.resource_id().raw();
                    let hd = ConnId::from(raw_id);
                    insert_connector(srv_net2.as_ref(), hd, &connector);
                }
                Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => {
                    log::error!(
                        "Could not connect to sock_addr: {}!!! error: {}!!!",
                        sock_addr,
                        err
                    );
                    (connector.connect_fn)(Err("ConnectionRefused".to_owned()));
                }
                Err(err) => {
                    log::error!(
                        "Could not connect to sock_addr: {}!!! error: {}!!!",
                        sock_addr,
                        err
                    );
                    (connector.connect_fn)(Err(err.to_string()));
                }
            }
        };

        // connect
        let srv_net3 = srv_net.clone();
        self.node_handler
            .connect(Transport::Tcp, sock_addr, move |_h, ret| {
                //
                srv_net3.run_in_service(Box::new(move || {
                    //
                    cb(ret);
                }));
            });
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
        self.node_handler.close(rid);
    }

    ///
    #[inline(always)]
    pub fn send_buffer(&self, hd: ConnId, sock_addr: SocketAddr, buffer: NetPacketGuard) {
        let rid = ResourceId::from(hd.id);
        let endpoint = Endpoint::new(rid, sock_addr);
        self.node_handler.send(endpoint, buffer);
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
        // multiplexor
        let mux = {
            let mut mux_opt_mut = self.mux_opt.write();
            mux_opt_mut.take().unwrap()
        };

        // callbacks
        let on_connect_ok = self.tcp_handler.on_connect_ok;
        let on_connect_err = self.tcp_handler.on_connect_err;
        let on_accept = self.tcp_handler.on_accept;
        let on_input = self.tcp_handler.on_input;
        let on_close = self.tcp_handler.on_close;

        //
        let srv_net = srv_net.clone();

        // read incoming network events.
        let node_task =
            node::node_listener_for_each_async(mux, &self.node_handler, move |_h, event| {
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
                            (on_connect_ok)(&srv_net, hd, os_addr);
                        } else {
                            // connect err
                            (on_connect_err)(&srv_net, hd);
                        }
                    }

                    NetEvent::Accepted(endpoint, id) => {
                        //
                        let raw_id = endpoint.resource_id().raw();
                        let hd = ConnId::from(raw_id);
                        let listener_id = ListenerId::from(id.raw());
                        /*log::info!(
                            "[hd={}] {} endpoint {} accepted, listener_id={}",
                            hd,
                            raw_id,
                            endpoint,
                            listener_id
                        );*/

                        //
                        (on_accept)(&srv_net, listener_id, hd, endpoint.addr().into());
                    } // NetEvent::Accepted

                    NetEvent::Message(endpoint, data) => {
                        let raw_id = endpoint.resource_id().raw();
                        let hd = ConnId::from(raw_id);

                        //
                        let input_buffer = InputBuffer { data };
                        (on_input)(&srv_net, hd, input_buffer);
                    }

                    NetEvent::Disconnected(endpoint) => {
                        //
                        let raw_id = endpoint.resource_id().raw();
                        let hd = ConnId::from(raw_id);
                        //log::info!("[hd={}] endpoint {} disconnected", hd, endpoint);

                        //
                        (on_close)(&srv_net, hd);
                    }
                }
            });

        //
        node_task
    }
}
