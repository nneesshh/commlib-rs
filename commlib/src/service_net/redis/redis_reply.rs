use super::super::{NetPacketGuard, PacketResult};

///
pub struct RedisReply {
    pkt_opt: Option<NetPacketGuard>, // 使用 option 以便 pkt 移交
}

impl RedisReply {
    ///
    pub fn new() -> Self {
        Self { pkt_opt: None }
    }
}
