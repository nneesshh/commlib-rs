use bytes::BytesMut;
use std::cell::RefCell;
use std::collections::LinkedList;
use std::rc::Rc;
use std::sync::Arc;

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
pub type EncryptTokenHander = Box<dyn Fn(&NetProxy, ConnId) + Send + Sync>;
pub type PacketHander = Box<dyn Fn(&NetProxy, ConnId, CmdId, &[u8]) + Send + Sync>;

///
pub struct NetProxy {
    packet_type: PacketType, // 通信 packet 类型
    srv_net: Arc<ServiceNetRs>,

    hd_encrypt_table: hashbrown::HashMap<ConnId, RefCell<EncryptData>>, // 每条连接的包序号和密钥（客户端连接才需要保存）
    encrypt_token_handler: EncryptTokenHander,

    default_handler: PacketHander,
    handlers: hashbrown::HashMap<CmdId, Rc<PacketHander>>,
}

impl NetProxy {
    ///
    pub fn new(packet_type: PacketType, srv_net: &Arc<ServiceNetRs>) -> NetProxy {
        NetProxy {
            packet_type,
            srv_net: srv_net.clone(),

            hd_encrypt_table: hashbrown::HashMap::new(),
            encrypt_token_handler: Box::new(|_1, _2| {}),

            default_handler: Box::new(|_1, _2, _3, _4| {}),
            handlers: hashbrown::HashMap::new(),
        }
    }

    ///
    pub fn on_incomming_conn(&mut self, hd: ConnId, push_encrypt_token: bool) {
        //
        let packet_type = self.packet_type();
        log::info!(
            "[hd={}] on_incomming_conn packet_type={:?}",
            hd,
            packet_type
        );
        hd.set_packet_type(self.srv_net.as_ref(), packet_type);

        //
        if push_encrypt_token {
            // 发送 EncryptToken
            (self.encrypt_token_handler)(self, hd);
        }
    }

    ///
    pub fn on_net_packet(&mut self, hd: ConnId, mut pkt: NetPacketGuard) {
        if pkt.decode_packet(hd, &mut self.hd_encrypt_table) {
            let cmd = pkt.cmd();
            if let Some(handler) = self.handlers.get(&cmd) {
                let h = handler.clone();
                let slice = pkt.consume();
                (h)(self, hd, cmd, slice);
            } else {
                // no-handler(trans), use default handler
                let slice = pkt.consume();
                (self.default_handler)(self, hd, cmd, slice);
            }
        }
    }

    ///
    #[inline(always)]
    pub fn packet_type(&self) -> PacketType {
        self.packet_type
    }

    ///
    pub fn set_encrypt_token_handler<F>(&mut self, f: F)
    where
        F: Fn(&NetProxy, ConnId) + Send + Sync + 'static,
    {
        self.encrypt_token_handler = Box::new(f);
    }

    ///
    pub fn set_encrypt_key(&mut self, hd: ConnId, key: Vec<u8>) {
        let encrypt_opt = self.hd_encrypt_table.get(&hd);
        if let Some(encrypt) = encrypt_opt {
            let old_encrypt = encrypt;
            log::error!(
                "set [hd={}] encrypt key error!!! already exists {}!!!",
                hd,
                Base64::encode(&old_encrypt.borrow().encrypt_key)
            );
            self.hd_encrypt_table.remove(&hd);
        }

        //
        log::info!("set [hd={}] encrypt key {}", hd, Base64::encode(&key));
        self.hd_encrypt_table.insert(
            hd,
            RefCell::new(EncryptData {
                no_list: LinkedList::new(),
                encrypt_key: unsafe { String::from_utf8_unchecked(key) },
            }),
        );
    }

    /// 发送接口线程安全
    #[inline(always)]
    pub fn send_raw(&self, hd: ConnId, cmd: CmdId, slice: &[u8]) {
        let mut pkt = take_packet(slice.len());
        pkt.set_type(self.packet_type);
        pkt.set_cmd(cmd);
        pkt.set_body(slice);

        //
        self.send_packet(hd, pkt);
    }

    //#[inline(always)]
    pub fn send_proto<M>(&self, hd: ConnId, cmd: CmdId, msg: &M)
    where
        M: prost::Message,
    {
        //
        let len = msg.encoded_len();
        let mut pkt = take_packet(len);
        pkt.set_type(self.packet_type);
        pkt.set_cmd(cmd);
        pkt.set_msg(msg);

        //
        self.send_packet(hd, pkt);
    }

    //#[inline(always)]
    fn send_packet(&self, hd: ConnId, mut pkt: NetPacketGuard) {
        if pkt.encode_packet(hd, &self.hd_encrypt_table) {
            let slice = pkt.consume();
            log::info!("send: {:?}", slice);
            hd.send(self.srv_net.as_ref(), slice);
        } else {
            log::error!("[hd={}] send packet failed!!!", hd);
            return;
        }
    }
}
