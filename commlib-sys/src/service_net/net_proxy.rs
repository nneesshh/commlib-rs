use bytes::BytesMut;
use std::collections::LinkedList;

use crate::service_net::net_packet::rand_packet_no;
use crate::{Base64, ServiceNetRs};

use super::take_packet;
use super::{CmdId, ConnId, EncryptData, NetPacketGuard, PacketType};

///
pub struct CrossRoutInfo {
    zone: crate::ZoneId,
    node: crate::NodeId,
    rpcid: u64,
    pid: crate::PlayerId,
}

///
pub type PacketHander = Box<dyn Fn(ConnId, CmdId, &[u8]) + Send + Sync>;

///
pub struct NetProxy {
    packet_type: PacketType, // 通信 packet 类型

    hd_encrypt_table: hashbrown::HashMap<ConnId, EncryptData>, // 每条连接的包序号和密钥（客户端连接才需要保存）

    default_handler: PacketHander,
    handlers: hashbrown::HashMap<CmdId, PacketHander>,
}

impl NetProxy {
    ///
    pub fn new(packet_type: PacketType) -> NetProxy {
        NetProxy {
            packet_type,
            hd_encrypt_table: hashbrown::HashMap::new(),

            default_handler: Box::new(|_1, _2, _3| {}),
            handlers: hashbrown::HashMap::new(),
        }
    }

    ///
    pub fn on_net_packet(&mut self, hd: ConnId, mut pkt: NetPacketGuard) {
        if pkt.decode_packet(hd, &mut self.hd_encrypt_table) {
            let cmd = pkt.cmd();
            if let Some(handler) = self.handlers.get_mut(&cmd) {
                let slice = pkt.consume();
                (handler)(hd, cmd, slice);
            } else {
                // no-handler(trans), use default handler
                let slice = pkt.consume();
                (self.default_handler)(hd, cmd, slice);
            }
        }
    }

    ///
    pub fn get_packet_type(&self) -> PacketType {
        self.packet_type
    }

    ///
    pub fn set_encrypt_key(&mut self, hd: ConnId, key: Vec<u8>) {
        let encrypt_opt = self.hd_encrypt_table.get(&hd);
        if let Some(encrypt) = encrypt_opt {
            let old_encrypt = encrypt;
            log::error!(
                "set [hd={}] encrypt key error!!! already exists {}!!!",
                hd,
                Base64::encode(&old_encrypt.encrypt_key)
            );
            self.hd_encrypt_table.remove(&hd);
        }

        //
        log::info!("set [hd={}] encrypt key {}", hd, Base64::encode(&key));
        self.hd_encrypt_table.insert(
            hd,
            EncryptData {
                no_list: LinkedList::new(),
                encrypt_key: unsafe { String::from_utf8_unchecked(key) },
            },
        );
    }

    ///
    #[inline(always)]
    pub fn get_encrypt_data(&self, hd: ConnId) -> Option<&EncryptData> {
        self.hd_encrypt_table.get(&hd)
    }

    /// 发送接口线程安全,
    #[inline(always)]
    pub fn send_raw(&mut self, srv_net: &ServiceNetRs, hd: ConnId, cmd: CmdId, slice: &[u8]) {
        let mut pkt = take_packet(slice.len(), self.packet_type);
        pkt.set_cmd(cmd);
        pkt.set_body(slice);

        //
        self.send_packet(srv_net, hd, pkt);
    }

    #[inline(always)]
    pub fn send_proto<M>(&mut self, srv_net: &ServiceNetRs, hd: ConnId, cmd: CmdId, msg: &M)
    where
        M: prost::Message,
    {
        //
        let len = msg.encoded_len();
        let mut pkt = take_packet(len, self.packet_type);
        pkt.set_cmd(cmd);
        pkt.set_msg(msg);

        //
        self.send_packet(srv_net, hd, pkt);
    }

    #[inline(always)]
    fn send_packet(&mut self, srv_net: &ServiceNetRs, hd: ConnId, mut pkt: NetPacketGuard) {
        if pkt.encode_packet(hd, &mut self.hd_encrypt_table) {
            let slice = pkt.consume();
            hd.send(srv_net, slice);
        } else {
            log::error!("[hd={}] send packet failed!!!", hd);
            return;
        }
    }
}
