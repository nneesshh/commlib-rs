use super::NetPacketGuard;

/// Read result
pub enum PacketResult {
    Ready(Vec<NetPacketGuard>), // pkt list
    Abort(String),
}
