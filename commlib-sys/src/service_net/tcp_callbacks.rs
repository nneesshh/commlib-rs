use super::net_packet::*;
use super::tcp_conn::*;
use crate::ServiceRs;
use std::sync::Arc;

#[repr(C)]
pub struct TcpHandler {
    pub on_listen: extern "C" fn(&'static dyn ServiceRs, String),
    pub on_accept: extern "C" fn(&'static dyn ServiceRs, Arc<TcpConn>),
    pub on_encrypt: extern "C" fn(&'static dyn ServiceRs, Arc<TcpConn>),

    pub on_connect: extern "C" fn(&'static dyn ServiceRs, Arc<TcpConn>),
    pub on_packet: extern "C" fn(&'static dyn ServiceRs, Arc<TcpConn>, Arc<NetPacket>),
    pub on_close: extern "C" fn(&'static dyn ServiceRs, Arc<TcpConn>),
}
