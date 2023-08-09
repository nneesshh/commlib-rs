//! Commlib: TcpServer

use super::net_packet::*;
use super::tcp_callbacks::*;
use super::tcp_conn::*;
use crate::ServiceRs;
use parking_lot::RwLock;
use std::rc::Rc;
use std::sync::{atomic::AtomicUsize, Arc};

lazy_static::lazy_static! {
    pub static ref TCP_SERVER_LIST: Arc<RwLock<Vec<TcpServer>>> = Arc::new(RwLock::new(Vec::new()));
}

///
pub struct TcpServer {
    start: std::time::Instant,

    handler: Option<TcpHandler>,

    conn_fn: Box<dyn FnMut(&TcpServer, &mut TcpHandler, &mut TcpConn) + Send + Sync + 'static>,
    msg_fn:
        Box<dyn FnMut(&TcpServer, &mut TcpHandler, &mut TcpConn, &[u8]) + Send + Sync + 'static>,

    stopped_cb: Box<dyn FnMut() + Send + Sync + 'static>,

    connection_limit: AtomicUsize,
    connection_num: AtomicUsize,

    inner_server: Option<MessageIoServer>,
}

impl TcpServer {
    ///
    pub fn new() -> TcpServer {
        TcpServer {
            start: std::time::Instant::now(),
            handler: None,

            conn_fn: Box::new(|_1, _2, _3| {}),
            msg_fn: Box::new(|_1, _2, _3, _4| {}),

            stopped_cb: Box::new(|| {}),

            connection_limit: AtomicUsize::new(0),
            connection_num: AtomicUsize::new(0),

            inner_server: None,
        }
    }

    ///
    pub fn on_connection<S>(&self, srv: &S, h: &mut TcpHandler, conn: &mut TcpConn, encrypt: bool)
    where
        S: ServiceRs,
    {
    }

    ///
    pub fn on_message<S>(&self, srv: &S, h: &mut TcpHandler, conn: &mut TcpConn, slice: &[u8])
    where
        S: ServiceRs,
    {
    }

    ///
    pub fn listen<S>(
        srv: &'static S,
        name: &str,
        ip: &str,
        port: u16,
        h: TcpHandler,
        thread_num: u32,
        connection_limit: u32,
    ) -> TcpServer
    where
        S: ServiceRs,
    {
        let mut tcp_server = TcpServer::new();
        let raddr = std::format!("{}:{}", ip, port);
        tcp_server.tcp_listen_addr(srv, name, raddr.as_ref(), h, thread_num, connection_limit);
        tcp_server
    }

    fn tcp_listen_addr<S>(
        &mut self,
        srv: &'static S,
        name: &str,
        addr: &str,
        h: TcpHandler,
        thread_num: u32,
        connection_limit: u32,
    ) where
        S: ServiceRs,
    {
        self.handler = Some(h);

        self.set_connection_callback(
            move |tcp_server: &TcpServer, h: &mut TcpHandler, conn: &mut TcpConn| {
                let encrypt = true; // tcp server 立即发送 EncryptToken
                tcp_server.on_connection(srv, h, conn, encrypt);
            },
        );

        self.set_message_callback(
            move |tcp_server: &TcpServer, h: &mut TcpHandler, conn: &mut TcpConn, slice: &[u8]| {
                let encrypt = true; // tcp server 立即发送 EncryptToken
                tcp_server.on_message(srv, h, conn, slice);
            },
        );

        //
        self.init(name);

        // trigger OnListen event in srv
        let on_listen = self.handler.as_ref().unwrap().on_listen.to_owned();
        let name = name.to_owned();
        srv.run_in_service(Box::new(move || {
            on_listen(srv, name.to_owned());
        }));
    }

    fn set_connection_callback<F>(&mut self, cb: F)
    where
        F: FnMut(&TcpServer, &mut TcpHandler, &mut TcpConn) + Send + Sync + 'static,
    {
        self.conn_fn = Box::new(cb);
    }

    fn set_message_callback<F>(&mut self, cb: F)
    where
        F: FnMut(&TcpServer, &mut TcpHandler, &mut TcpConn, &[u8]) + Send + Sync + 'static,
    {
        self.msg_fn = Box::new(cb);
    }

    fn init(&mut self, addr: &str) -> bool {
        let inner = MessageIoServer::new(addr).unwrap();
        self.inner_server = Some(inner);
        //se
        true
    }
}

struct MessageIoServer {
    handler: message_io::node::NodeHandler<()>,
    node_listener: Option<message_io::node::NodeListener<()>>,
}

impl MessageIoServer {
    pub fn new(listen_addr: &str) -> std::io::Result<MessageIoServer> {
        // Create a node, the main message-io entity. It is divided in 2 parts:
        // The 'handler', used to make actions (connect, send messages, signals, stop the node...)
        // The 'listener', used to read events from the network or signals.
        let (handler, node_listener) = message_io::node::split::<()>();

        handler
            .network()
            .listen(message_io::network::Transport::Tcp, listen_addr)?;

        log::info!("server running at {}", listen_addr);

        Ok(MessageIoServer {
            handler,
            node_listener: Some(node_listener),
        })
    }

    pub fn run(mut self) {
        let node_listener = self.node_listener.take().unwrap();

        // Read incoming network events.
        node_listener.for_each(move |event| match event.network() {
            message_io::network::NetEvent::Connected(_, _) => unreachable!(), // There is no connect() calls.
            message_io::network::NetEvent::Accepted(_, _) => (), // All endpoint accepted
            message_io::network::NetEvent::Message(endpoint, input_data) => {}
            message_io::network::NetEvent::Disconnected(endpoint) => {}
        });
    }
}
