//!
//! Common Library: service-net
//!

///
pub mod buffer;
pub use buffer::Buffer;

///
pub mod net_packet;
pub use net_packet::{AtomicPacketType, CmdId, EncryptData, NetPacket, PacketType};
pub use net_packet::{ENCRYPT_MAX_LEN, ENCRYPT_KEY_LEN};

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
pub mod tcp_listener_id;
pub use tcp_listener_id::TcpListenerId;

///
pub mod net_proxy;
pub use net_proxy::NetProxy;

///
pub mod tcp_handler;
pub use tcp_handler::TcpHandler;

///
pub mod tcp_conn;
pub use tcp_conn::TcpConn;

///
pub mod tcp_server;
pub use tcp_server::TcpServer;

///
pub mod tcp_client;
pub use tcp_client::TcpClient;

///
pub mod server_status;
pub use server_status::{AtomicServerStatus, ServerStatus};

///
pub mod client_status;
pub use client_status::{AtomicClientStatus, ClientStatus};

pub mod os_socket_addr;
pub use os_socket_addr::OsSocketAddr;

///
pub mod service_net_impl;
pub use service_net_impl::*;

///
pub mod network_impl;
pub use network_impl::*;

///
pub mod connect_to_server_helper;
pub use connect_to_server_helper::*;
