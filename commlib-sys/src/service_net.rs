//!
//! Common Library: service-net
//!

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
pub mod packet_reader;
pub use packet_reader::PacketReader;

///
pub mod conn_id;
pub use conn_id::ConnId;

///
pub mod net_proxy;
pub use net_proxy::NetProxy;

///
pub mod tcp_callbacks;
pub use tcp_callbacks::{ServerCallbacks, TcpClientHandler, TcpServerHandler};

///
pub mod tcp_conn;
pub use tcp_conn::TcpConn;

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

///
pub mod service_net_impl;
pub use service_net_impl::ServiceNetRs;
