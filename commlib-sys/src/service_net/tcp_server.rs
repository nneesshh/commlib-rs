//! Commlib: TcpServer
//! We can use this class to create a TCP server.
//! The typical usage is :
//!      1. Create a TCPServer object
//!      2. Set the message callback and connection callback
//!      3. Call TCPServer::Init()
//!      4. Call TCPServer::Start()
//!      5. Process TCP client connections and messages in callbacks
//!      6. At last call Server::Stop() to stop the whole server
//!
//! The example code is as bellow:
//! //<code>
//! # Example
//! ```
//!     std::string addr = "0.0.0.0:9099";
//!     int thread_num = 4;
//!     evpp::EventLoop loop;
//!     evpp::TCPServer server(&loop, addr, "TCPEchoServer", thread_num);
//!     server.SetMessageCallback([](const evpp::TCPConnPtr& conn,
//!                                  evpp::Buffer* msg) {
//!         // Do something with the received message
//!         conn->Send(msg); // At here, we just send the received message back.
//!     });
//!     server.SetConnectionCallback([](const evpp::TCPConnPtr& conn) {
//!         if (conn->IsConnected()) {
//!             LOG_INFO << "A new connection from " << conn->remote_addr();
//!         } else {
//!             LOG_INFO << "Lost the connection from " << conn->remote_addr();
//!         }
//!     });
//!     server.Init();
//!     server.Start();
//!     loop.Run();
//! ```
//! //</code>
//!

use super::MessageIoServer;
use super::{ServerCallbacks, TcpServerHandler};
use super::{ServerStatus, ServerSubStatus};

use crate::G_SERVICE_NET;
use parking_lot::RwLock;

use std::sync::atomic::AtomicUsize;
///
#[repr(C)]
pub struct TcpServer {
    start: std::time::Instant,
    status: RwLock<ServerStatus>,
    substatus: RwLock<ServerSubStatus>,

    connection_limit: AtomicUsize,
    connection_num: AtomicUsize,

    inner_server: Option<MessageIoServer>,
    pub callbacks: ServerCallbacks,
}

impl TcpServer {
    ///
    pub fn new() -> TcpServer {
        TcpServer {
            start: std::time::Instant::now(),
            status: RwLock::new(ServerStatus::Null),
            substatus: RwLock::new(ServerSubStatus::SubStatusNull),

            connection_limit: AtomicUsize::new(0),
            connection_num: AtomicUsize::new(0),

            inner_server: None,
            callbacks: ServerCallbacks::new(),
        }
    }

    ///
    pub fn init(&mut self) -> bool {
        let mut status_mut = self.status.write();

        // inner server
        let tcp_server_handler = TcpServerHandler::new();
        let tcp_server_id = self as *const TcpServer as usize;
        self.inner_server = Some(MessageIoServer::new(tcp_server_handler, tcp_server_id));

        // initialize finish
        (*status_mut) = ServerStatus::Initialized;
        true
    }

    ///
    pub fn start(&mut self, addr: &str) {
        let inner_server = self.inner_server.as_mut().unwrap();

        // server prepare
        {
            let mut status_mut = self.status.write();

            assert_eq!((*status_mut) as u32, ServerStatus::Initialized as u32);
            (*status_mut) = ServerStatus::Starting;

            // TODO:

            (*status_mut) = ServerStatus::Running;
        }

        // server listen
        inner_server.listen(addr, G_SERVICE_NET.as_ref()).unwrap();

        // server loop
        inner_server.run(G_SERVICE_NET.as_ref());
    }

    ///
    pub fn listen(
        ip: &str,
        port: u16,
        thread_num: u32,
        connection_limit: u32,
        callbacks: ServerCallbacks,
    ) -> TcpServer {
        let mut tcp_server = TcpServer::new();

        //
        {
            let raddr = std::format!("{}:{}", ip, port);
            tcp_server.tcp_listen_addr(raddr.as_ref(), thread_num, connection_limit, callbacks);
            tcp_server
        }
    }

    fn tcp_listen_addr(
        &mut self,
        addr: &str,
        _thread_num: u32,       // TODO:
        _connection_limit: u32, // TODO:
        callbacks: ServerCallbacks,
    ) {
        // init callbacks
        unsafe {
            let callbacks_mut = &self.callbacks as *const ServerCallbacks as *mut ServerCallbacks;
            (*callbacks_mut) = callbacks;
        }

        //
        self.init();

        //
        self.start(addr);
    }
}
