use crate::service_net::TcpConn;
use std::collections::LinkedList;
use std::rc::Rc;

use crate::Base64;

use super::{decode_packet, encode_packet, get_leading_field_size, take_packet};
use super::{CmdId, ConnId, EncryptData, NetPacketGuard, PacketType};

///
pub struct CrossRoutInfo {
    zone: crate::ZoneId,
    node: crate::NodeId,
    rpcid: u64,
    pid: crate::PlayerId,
}

///
pub type EncryptTokenHander = Box<dyn Fn(&mut NetProxy, &TcpConn) + Send + Sync>;
pub type PacketHander = Box<dyn Fn(&mut NetProxy, &TcpConn, CmdId, &[u8]) + Send + Sync>;

///
pub struct NetProxy {
    packet_type: PacketType, // 通信 packet 类型
    leading_field_size: u8,  // 包体前导长度字段占用字节数

    hd_encrypt_table: hashbrown::HashMap<ConnId, EncryptData>, // 每条连接的包序号和密钥
    encrypt_token_handler: Rc<EncryptTokenHander>,

    default_handler: Rc<PacketHander>,
    handlers: hashbrown::HashMap<CmdId, Rc<PacketHander>>,
}

impl NetProxy {
    ///
    pub fn new(packet_type: PacketType) -> NetProxy {
        let leading_field_size = get_leading_field_size(packet_type);

        NetProxy {
            packet_type,
            leading_field_size,

            hd_encrypt_table: hashbrown::HashMap::with_capacity(4096),
            encrypt_token_handler: Rc::new(Box::new(|_1, _2| {})),

            default_handler: Rc::new(Box::new(|_1, _2, _3, _4| {})),
            handlers: hashbrown::HashMap::new(),
        }
    }

    ///
    pub fn on_incomming_conn(&mut self, conn: &TcpConn, push_encrypt_token: bool) {
        let hd = conn.hd;
        //
        log::info!(
            "[hd={}] on_incomming_conn  packet_type={:?}",
            hd,
            self.packet_type
        );
        conn.set_packet_type(self.packet_type);

        //
        if push_encrypt_token {
            // 发送 EncryptToken
            let encrypt_token_handler = self.encrypt_token_handler.clone();
            (*encrypt_token_handler)(self, conn);
        }
    }

    ///
    pub fn on_net_packet(&mut self, conn: &TcpConn, mut pkt: NetPacketGuard) {
        let hd = conn.hd;
        {
            let peek = pkt.peek();
            log::info!("[hd={}] on_net_packet: 1) {:?}", hd, peek);
        }

        if decode_packet(self.packet_type, hd, &mut pkt, &mut self.hd_encrypt_table) {
            let cmd = pkt.cmd();
            let slice = pkt.consume();
            log::info!("[hd={}] on_net_packet: 2) cmd({})", hd, cmd);

            if let Some(handler) = self.handlers.get(&cmd) {
                let h = handler.clone();
                (h)(self, conn, cmd, slice);
            } else {
                // no-handler(trans), use default handler
                let default_handler = self.default_handler.clone();
                (*default_handler)(self, conn, cmd, slice);
            }
        } else {
            //
            let peek = pkt.peek();
            log::error!("[hd={}] on_net_packet failed!!! {:?}!!!", hd, peek);
        }
    }

    ///
    pub fn on_hd_lost(&mut self, _hd: ConnId) {
        // TODO:
    }

    ///
    pub fn set_encrypt_key(&mut self, hd: ConnId, key: &[u8]) {
        let key = unsafe { String::from_utf8_unchecked(key.to_vec()) };

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
        log::info!(
            "set [hd={}] encrypt key {} len {}",
            hd,
            Base64::encode(&key),
            key.len()
        );
        self.hd_encrypt_table.insert(
            hd,
            EncryptData {
                no_list: LinkedList::new(),
                encrypt_key: key,
            },
        );
    }

    ///
    #[inline(always)]
    pub fn packet_type(&self) -> PacketType {
        self.packet_type
    }

    ///
    pub fn set_encrypt_token_handler<F>(&mut self, f: F)
    where
        F: Fn(&mut NetProxy, &TcpConn) + Send + Sync + 'static,
    {
        self.encrypt_token_handler = Rc::new(Box::new(f));
    }

    /// cmd handler
    pub fn set_packet_handler<F>(&mut self, cmd: CmdId, f: F)
    where
        F: Fn(&mut NetProxy, &TcpConn, CmdId, &[u8]) + Send + Sync + 'static,
    {
        self.handlers.insert(cmd, Rc::new(Box::new(f)));
    }

    /// 发送接口线程安全
    #[inline(always)]
    pub fn send_raw(&mut self, conn: &TcpConn, cmd: CmdId, slice: &[u8]) {
        let mut pkt = take_packet(slice.len(), self.leading_field_size);
        pkt.set_cmd(cmd);
        pkt.set_body(slice);

        //
        self.send_packet(conn, pkt);
    }

    ///
    #[inline(always)]
    pub fn send_proto<M>(&mut self, conn: &TcpConn, cmd: CmdId, msg: &M)
    where
        M: prost::Message,
    {
        //
        let len = msg.encoded_len();
        let mut pkt = take_packet(len, self.leading_field_size);
        pkt.set_cmd(cmd);
        pkt.set_msg(msg);

        //
        self.send_packet(conn, pkt);
    }

    ///
    #[inline(always)]
    pub fn send_packet(&mut self, conn: &TcpConn, mut pkt: NetPacketGuard) {
        let hd = conn.hd;
        let cmd = pkt.cmd();

        //
        if encode_packet(conn.packet_type(), hd, &mut pkt, &mut self.hd_encrypt_table) {
            let slice = pkt.consume();
            log::info!("[hd={}] send packet cmd({}) -- {:?}", hd, cmd, slice);
            conn.send(slice);
        } else {
            //
            let peek = pkt.peek();
            log::error!(
                "[hd={}] send packet failed!!! cmd({})!!! {:?}!!!",
                hd,
                cmd,
                peek
            );
        }
    }
}
