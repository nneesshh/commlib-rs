//!
//! Common Library: service-net
//!

///
mod conn_id;
pub use conn_id::ConnId;

///
pub mod packet;
pub use packet::get_leading_field_size;
pub use packet::PacketType;
pub use packet::{FROM_CLIENT_PKT_LEADING_FIELD_SIZE, PKT_LEADING_FIELD_SIZE_DEFAULT};

///
mod listener_id;
pub use listener_id::ListenerId;

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
pub use tcp_server::tcp_server_listen;
pub use tcp_server::TcpServer;

///
pub mod connector;

mod listener;
mod packet_builder;

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
mod dns_resolver;
pub use dns_resolver::dns_resolve;

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

///
#[cfg(feature = "websocket")]
mod ws_server;
#[cfg(feature = "websocket")]
pub use ws_server::ws_server_listen;
#[cfg(feature = "websocket")]
pub use ws_server::WsServer;
