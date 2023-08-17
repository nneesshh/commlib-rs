use super::{net_packet::NetPacket, PacketType};
use crate::service_net::net_packet::PacketSizeType;
use lazy_static::lazy_static;
use opool::{Pool, PoolAllocator, RefGuard};

///
pub const SMALL_PACKET_MAX_SIZE: usize = 1024 * 4;

///
pub type NetPacketGuard = RefGuard<'static, NetPacketPool, NetPacket>;

///
pub struct NetPacketPool;

impl PoolAllocator<NetPacket> for NetPacketPool {
    #[inline]
    fn allocate(&self) -> NetPacket {
        let mut pkt = NetPacket::new();
        pkt.init(true);
        pkt
    }

    /// OPTIONAL METHODS:
    #[inline]
    fn reset(&self, pkt: &mut NetPacket) {
        pkt.release();
    }

    #[inline]
    fn is_valid(&self, pkt: &NetPacket) -> bool {
        // you can optionally is_valid if object is good to be pushed back to the pool
        true
    }
}

///
#[inline(always)]
pub fn take_packet(size: usize, packet_type: PacketType) -> NetPacketGuard {
    if size <= SMALL_PACKET_MAX_SIZE {
        let mut pkt = G_PACKET_POOL_SMALL.get();
        pkt.set_type(packet_type);
        pkt
    } else {
        let mut pkt = G_PACKET_POOL_LARGE.get();
        pkt.set_size_type(PacketSizeType::Large);
        pkt.set_type(packet_type);
        pkt
    }
}

#[inline(always)]
pub fn take_larget_packet(
    ensure_bytes: usize,
    packet_type: PacketType,
    init_slice: &[u8],
) -> NetPacketGuard {
    let mut pkt = G_PACKET_POOL_LARGE.get();
    pkt.set_type(packet_type);
    pkt.ensure_writable_bytes(ensure_bytes);
    pkt.append_slice(init_slice);
    pkt
}

lazy_static! {
    // < 4k (SMALL_PACKEG_MAX_SIZE), 最多 cache 8192个
    pub static ref G_PACKET_POOL_LARGE: Pool<NetPacketPool, NetPacket> = Pool::new(8192, NetPacketPool);

    // >= 4k (SMALL_PACKEG_MAX_SIZE), 最多 cache 128个, 超过数量上限立即释放，避免占用过多内存
    pub static ref G_PACKET_POOL_SMALL: Pool<NetPacketPool, NetPacket> = Pool::new(128, NetPacketPool);
}
