//!
//! Common Library: service-signal
//!
use parking_lot::RwLock;

pub use super::commlib_service::*;

///
pub mod buffer;
pub use buffer::Buffer;

///
pub mod net_packet;
pub use net_packet::{CmdId, EncryptData, NetPacket, PacketType};

///
pub mod net_packet_pool;
pub use net_packet_pool::{take_packet, NetPacketGuard, NetPacketPool};

///
pub mod net_proxy;
pub use net_proxy::NetProxy;

///
pub mod tcp_callbacks;
pub use tcp_callbacks::{ServerCallbacks, TcpClientHandler, TcpServerHandler};

///
pub mod tcp_conn;
pub use tcp_conn::{ConnId, TcpConn};

///
pub mod tcp_server;
pub use tcp_server::TcpServer;

///
pub mod server_status;
pub use server_status::{ServerStatus, ServerSubStatus};

///
pub mod server_impl;
pub use server_impl::*;

pub mod os_socketaddr;
pub use os_socketaddr::OsSocketAddr;

/// ServiceNetRs
pub struct ServiceNetRs {
    pub handle: RwLock<ServiceHandle>,

    pub conn_table: RwLock<hashbrown::HashMap<ConnId, TcpConn>>, // TODO: remove lock?
    pub server_table: RwLock<hashbrown::HashMap<usize, TcpServer>>, // TODO: remove lock?
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64) -> ServiceNetRs {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, NodeState::Idle)),

            conn_table: RwLock::new(hashbrown::HashMap::with_capacity(4096)),
            server_table: RwLock::new(hashbrown::HashMap::new()),
        }
    }

    ///
    pub fn send(&self, hd: ConnId, data: &[u8]) {
        let conn_table = self.conn_table.read();
        if let Some(&tcp_conn) = conn_table.get(&hd) {
            tcp_conn.send(data);
        }
    }

    ///
    pub fn listen(
        &self,
        ip: &str,
        port: u16,
        thread_num: u32,
        connection_limit: u32,
        callbacks: ServerCallbacks,
    ) {
        // tcp server
        {
            let tcp_server = TcpServer::listen(ip, port, thread_num, connection_limit, callbacks);
            {
                let mut server_table_mut = self.server_table.write();
                let tcp_server_id = &tcp_server as *const TcpServer as usize;
                (*server_table_mut).insert(tcp_server_id, tcp_server);
            }
        }
    }
}

impl ServiceRs for ServiceNetRs {
    /// 获取 service nmae
    fn name(&self) -> &str {
        "service_net"
    }

    /// 获取 service 句柄
    fn get_handle(&self) -> &RwLock<ServiceHandle> {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

    /// Init in-service
    fn init(&self) -> bool {
        true
    }

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnMut() + Send + Sync + 'static>) {
        let handle = self.get_handle().read();
        handle.run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool {
        let handle = self.get_handle().read();
        handle.is_in_service_thread()
    }

    /// 等待线程结束
    fn join(&self) {
        let mut handle_mut = self.get_handle().write();
        handle_mut.join_service();
    }
}
