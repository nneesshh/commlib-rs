//!
//! Common Library: service-net
//!

///
mod buffer;
pub use buffer::Buffer;

///
mod net_packet;
pub use net_packet::PKT_CMD_LEN;
pub use net_packet::{CmdId, EncryptData, NetPacket, PacketType};

///
mod net_packet_encdec;
pub use net_packet_encdec::{ENCRYPT_KEY_LEN, ENCRYPT_MAX_LEN};

///
mod net_packet_pool;
pub use net_packet_pool::{NetPacketGuard, NetPacketPool};

///
mod packet_builder;

///
mod conn_id;
pub use conn_id::ConnId;

///
mod listener_id;
pub use listener_id::ListenerId;

///
mod net_proxy;
pub use net_proxy::NetProxy;

///
mod tcp_handler;
pub use tcp_handler::TcpHandler;

///
mod tcp_conn;
pub use tcp_conn::TcpConn;

///
mod tcp_conn_manager;
pub use tcp_conn_manager::disconnect_connection;

///
mod tcp_server;
pub use tcp_server::TcpServer;

///
mod tcp_server_manager;
pub use tcp_server_manager::tcp_server_listen;

///
pub mod connector;

///
mod listener;

///
mod tcp_client;
pub use tcp_client::TcpClient;

///
mod tcp_client_manager;
pub use tcp_client_manager::{connect_to_tcp_server, remove_tcp_client};

///
mod server_status;
pub use server_status::ServerStatus;

///
mod client_status;
pub use client_status::ClientStatus;

///
mod os_socket_addr;
pub use os_socket_addr::OsSocketAddr;

///
mod low_level_network;
pub use low_level_network::MessageIoNetwork;

///
mod service_net_impl;
pub use service_net_impl::ServiceNetRs;
pub use service_net_impl::{start_network, stop_network};

///
mod redis;
pub use redis::cmds as redis_cmds;
pub use redis::connect_to_redis;
pub use redis::{RedisClient, RedisCommander, RedisReply, RedisReplyType};

///
mod http_server;
pub use http_server::HttpServer;
pub use http_server::{http_server_listen, parsing as http_parsing};
