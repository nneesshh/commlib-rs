///
mod buffer;
pub use buffer::Buffer;

///
mod net_packet_impl;
pub use net_packet_impl::{CmdId, NetPacket};

///
mod net_packet_pool;
pub use net_packet_pool::NetPacketGuard;
pub use net_packet_pool::SMALL_PACKET_MAX_SIZE;
pub use net_packet_pool::{take_large_packet, take_packet, take_small_packet};
