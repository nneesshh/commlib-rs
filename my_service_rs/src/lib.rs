//! Commlib: event, log, service, ...

///

///
mod service;
pub use service::*;

///
mod service_signal;
pub use service_signal::ServiceSignalRs;

///
mod service_net;
pub use service_net::http_parsing;
#[cfg(feature = "websocket")]
pub use service_net::ws_server_listen;
pub use service_net::{
    connect_to_redis, connect_to_tcp_server, get_leading_field_size, http_server_listen,
    start_network, stop_network, tcp_server_listen,
};
pub use service_net::{redis_cmds as redis, RedisClient, RedisReply, RedisReplyType};
pub use service_net::{
    ClientStatus, ConnId, ListenerId, OsSocketAddr, PacketType, ServerStatus, ServiceNetRs,
    TcpClient, TcpConn, TcpServer,
};
pub use service_net::{FROM_CLIENT_PKT_LEADING_FIELD_SIZE, PKT_LEADING_FIELD_SIZE_DEFAULT};

///
mod service_http_client;
pub use service_http_client::{HttpRequestType, ServiceHttpClientRs};

/// 全局变量
mod globals;
pub use globals::*;
