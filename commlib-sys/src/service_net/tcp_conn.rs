use crate::ServiceNetRs;
use message_io::network::{Endpoint, NetworkController, ResourceId};
use std::net::SocketAddr;

use super::{
    net_packet_pool::SMALL_PACKET_MAX_SIZE, take_packet, Buffer, NetPacketGuard, PacketType,
    TcpServer,
};

const DEFAULT_PACKET_SIZE: usize = SMALL_PACKET_MAX_SIZE;

/// Connection id
#[derive(Debug, Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct ConnId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl ConnId {
    ///
    #[inline(always)]
    pub fn send(&self, srv_net: &'static ServiceNetRs, data: &[u8]) {
        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                tcp_conn.send(data);
            } else {
                log::error!("[hd={:?}] send failed -- hd not found!!!", *self);
            }
        }
    }

    ///
    #[inline(always)]
    pub fn send_proto<M>(&self, srv_net: &'static ServiceNetRs, msg: &M)
    where
        M: prost::Message,
    {
        let vec = msg.encode_to_vec();

        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                tcp_conn.send(vec.as_slice());
            } else {
                log::error!("[hd={:?}] send_proto failed -- hd not found!!!", *self);
            }
        }
    }

    ///
    pub fn to_socket_addr(&self, srv_net: &'static ServiceNetRs) -> Option<SocketAddr> {
        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(tcp_conn) = conn_table.get(self) {
                let id = ResourceId::from(self.id);
                Some(tcp_conn.endpoint.addr())
            } else {
                log::error!("[hd={:?}] to_socket_addr failed -- hd not found!!!", *self);
                None
            }
        }
    }
}

impl From<usize> for ConnId {
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}

///
pub enum PacketReaderState {
    Leading,     // 包体前导长度
    Data(usize), // 包体数据
}

impl PacketReaderState {
    ///
    pub fn read(
        &mut self,
        pkt: &mut NetPacketGuard,
        input_data: *const u8,
        input_len: usize,
    ) -> (bool, usize) {
        let leading_field_size = pkt.leading_field_size();
        let mut consumed = 0_usize;
        let mut remain = input_len;
        let mut pkt_full_len = 0_usize;

        while remain > 0 {
            match self {
                PacketReaderState::Leading => {
                    // 包体前导长度字段是否完整？
                    if pkt.buffer_raw_len() >= leading_field_size {
                        // 查看取包体前导长度
                        pkt_full_len = pkt.peek_leading_field();

                        // 转入包体数据处理
                        *self = PacketReaderState::Data(pkt_full_len - leading_field_size);
                        continue;
                    } else {
                        let need_bytes = leading_field_size - pkt.buffer_raw_len();
                        let append_bytes = if remain >= need_bytes {
                            need_bytes
                        } else {
                            remain
                        };

                        //
                        let off = (input_len - remain) as isize;
                        let ptr = unsafe { input_data.offset(off) };
                        pkt.append(ptr, append_bytes);
                        remain -= append_bytes;
                        consumed += append_bytes;
                    }
                }

                PacketReaderState::Data(need_bytes) => {
                    // 包体数据是否完整？
                    if pkt.buffer_raw_len() > *need_bytes {
                        std::unreachable!()
                    } else if pkt.buffer_raw_len() == *need_bytes {
                        // 完成当前 pkt 读取，重新开始包体前导长度处理，并把剩余长度返回外部
                        *self = PacketReaderState::Leading;
                        return (true, remain);
                    } else {
                        let append_bytes = if remain >= *need_bytes {
                            *need_bytes
                        } else {
                            remain
                        };

                        //
                        let off = (input_len - remain) as isize;
                        let ptr = unsafe { input_data.offset(off) };
                        pkt.append(ptr, append_bytes);
                        remain -= append_bytes;
                        consumed += append_bytes;
                    }
                }
            }
        }

        // 包体不完整
        (false, remain)
    }
}

/// Tcp connection
#[repr(C)]
pub struct TcpConn {
    //
    pub packet_type: PacketType,
    pub hd: ConnId,

    //
    pub endpoint: Endpoint,
    pub netctrl_id: usize,

    //
    pub input_pkt_opt: Option<NetPacketGuard>,
}

impl TcpConn {
    ///
    pub fn new(hd: ConnId, endpoint: Endpoint, netctrl: &NetworkController) -> TcpConn {
        let packet_type = PacketType::Server;
        let netctrl_id = netctrl as *const NetworkController as usize;

        TcpConn {
            packet_type,
            hd,
            endpoint,
            netctrl_id,
            input_pkt_opt: Some(take_packet(DEFAULT_PACKET_SIZE, packet_type)),
        }
    }

    ///
    pub fn handle_read(
        &self,
        srv_net: &'static ServiceNetRs,
        tcp_server: &TcpServer,
        hd: ConnId,
        slice: &[u8],
    ) {
        let mut consumed = 0_usize;
        let mut remain = slice.len();

        let pkt = self.input_pkt_opt.as_ref().unwrap();
        let leading_field_size = pkt.leading_field_size();

        // 检查包头是否完备
        /*while remain >= pkt_header_len_size {
            let len = if self.packet_type == PacketType::Client {
                2
            } else {
                4
            };
        }*/
    }

    ///
    pub fn send(&self, data: &[u8]) {
        log::debug!("[hd={:?}] send data ...", self.hd);

        let netctrl = unsafe { &*(self.netctrl_id as *const NetworkController) };
        netctrl.send(self.endpoint, data);
    }
}
