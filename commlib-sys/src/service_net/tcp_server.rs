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

use super::start_message_io_server_async;
use super::MessageIoServer;
use super::{ServerCallbacks, TcpServerHandler};
use super::{ServerStatus, ServerSubStatus};

use crate::ServiceNetRs;
use parking_lot::RwLock;

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

///
#[repr(C)]
pub struct TcpServer {
    start: std::time::Instant,
    status: RwLock<ServerStatus>,
    substatus: RwLock<ServerSubStatus>,

    connection_limit: AtomicUsize,
    connection_num: AtomicUsize,

    pub inner_server_opt: Arc<RwLock<Option<MessageIoServer>>>,
    pub callbacks_opt: Option<ServerCallbacks>,
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

            inner_server_opt: Arc::new(RwLock::new(None)),
            callbacks_opt: None,
        }
    }

    /// Create a tcp server and listen on [ip:port]
    pub fn listen(
        &'static self,
        ip: String,
        port: u16,
        callbacks: ServerCallbacks,
        srv_net: &'static ServiceNetRs,
    ) {
        let raddr = std::format!("{}:{}", ip, port);
        log::info!("tcp server listen raddr: {}", raddr);

        // set callbacks forcely
        unsafe {
            let callbacks_opt_mut = &self.callbacks_opt as *const Option<ServerCallbacks>
                as *mut Option<ServerCallbacks>;
            (*callbacks_opt_mut) = Some(callbacks);
        }

        // init inner-server
        self.inner_server_init();

        // inner-server listen
        self.inner_server_listen(raddr.as_str(), srv_net);
    }

    /// Start net event loop
    pub fn start(&'static self, srv_net: &'static ServiceNetRs) {
        log::info!(
            "tcp server start at {:?} status={} conn_num={}...",
            self.start,
            self.status.read().to_string(),
            self.connection_num
                .load(std::sync::atomic::Ordering::Relaxed),
        );
        self.inner_start(srv_net);
    }

    /// Stop net event loop
    pub fn stop(&self) {
        self.inner_stop();
    }

    fn inner_server_init(&self) -> bool {
        // inner server
        let tcp_server_handler = TcpServerHandler::new();
        let tcp_server_id = self as *const TcpServer as usize;

        // mount inner server opt
        {
            let mut inner_server_opt_mut = self.inner_server_opt.write();
            (*inner_server_opt_mut) = Some(MessageIoServer::new(tcp_server_handler, tcp_server_id));
        }

        // initialize finish
        {
            let mut status_mut = self.status.write();
            (*status_mut) = ServerStatus::Initialized;
        }
        true
    }

    fn inner_server_listen(&self, addr: &str, srv_net: &'static ServiceNetRs) {
        // server prepare
        {
            let mut status_mut = self.status.write();

            assert_eq!((*status_mut) as u32, ServerStatus::Initialized as u32);
            (*status_mut) = ServerStatus::Starting;

            // TODO:

            (*status_mut) = ServerStatus::Running;
        }

        // inner server listen
        {
            let mut inner_server_opt_mut = self.inner_server_opt.write();
            (*inner_server_opt_mut)
                .as_mut()
                .unwrap()
                .listen(addr, srv_net)
                .unwrap();
        }
    }

    fn inner_start(&'static self, srv_net: &'static ServiceNetRs) {
        // inner server run in async mode -- loop in a isolate thread
        start_message_io_server_async(&self.inner_server_opt, srv_net);
    }

    fn inner_stop(&self) {
        // inner server stop
        let mut inner_server_opt_mut = self.inner_server_opt.write();
        (*inner_server_opt_mut).as_mut().unwrap().stop();
    }
}
