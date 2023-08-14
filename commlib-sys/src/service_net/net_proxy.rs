use super::{take_packet, CmdId, ConnId, EncryptData, NetPacketGuard, PacketType};

///
pub struct CrossRoutInfo {
    zone: crate::ZoneId,
    node: crate::NodeId,
    rpcid: u64,
    pid: crate::PlayerId,
}

///
pub type PacketHander = Box<dyn FnMut(ConnId, CmdId, &[u8])>;

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

    /// 发送接口线程安全,
    pub fn send_raw(&mut self, hd: ConnId, cmd: CmdId, slice: &[u8]) {
        let guard = take_packet(slice.len(), self.packet_type, slice);
        self.send_packet(hd, guard);
    }

    #[inline(always)]
    fn send_packet(&mut self, hd: ConnId, mut pkt: NetPacketGuard) {
        pkt.encode_packet(hd, &mut self.hd_encrypt_table);
    }
}
