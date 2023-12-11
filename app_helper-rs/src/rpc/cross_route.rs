use commlib::{NodeId, PlayerId, ZoneId};

///
pub struct CrossRoute {
    zone: ZoneId,
    node: NodeId,
    rpcid: u64,
    pid: PlayerId,
}
