use std::rc::Rc;

pub struct CrossRoutInfo {
    zone: crate::ZoneId,
    node: crate::NodeId,
    rpcid: u64,
    pid: crate::PlayerId,
}
pub struct NetProxy {
    packet_type: u32,
}

impl NetProxy {
    ///
    pub fn new() -> NetProxy {
        NetProxy { packet_type: 0 }
    }
}
