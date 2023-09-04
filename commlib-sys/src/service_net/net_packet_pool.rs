use lazy_static::lazy_static;
use opool::{Pool, PoolAllocator, RefGuard};

use super::net_packet::{NetPacket, PacketSizeType};
use super::net_packet::{BUFFER_INITIAL_SIZE, BUFFER_RESERVED_PREPEND_SIZE};

/// packet 初始内存分配量
pub const SMALL_PACKET_MAX_SIZE: usize = BUFFER_INITIAL_SIZE - BUFFER_RESERVED_PREPEND_SIZE;
pub const LARGE_BUFFER_INITIAL_SIZE: usize = BUFFER_INITIAL_SIZE * 4;

/// 线程安全 packet
pub type NetPacketGuard = RefGuard<'static, NetPacketPool, NetPacket>;

lazy_static! {
    // < 4k (SMALL_PACKEG_MAX_SIZE), 最多 cache 8192个
    pub static ref G_PACKET_POOL_SMALL: Pool<NetPacketPool, NetPacket> = {
        Pool::new(8192, NetPacketPool)
    };

    // >= 4k (SMALL_PACKEG_MAX_SIZE), 最多 cache 128个, 超过数量上限立即释放，避免占用过多内存
    pub static ref G_PACKET_POOL_LARGE: Pool<NetPacketPool, NetPacket> = {
        Pool::new(128, NetPacketPool)
    };
}

///
pub struct NetPacketPool;

impl PoolAllocator<NetPacket> for NetPacketPool {
    ///
    //#[inline(always)]
    fn allocate(&self) -> NetPacket {
        let mut pkt = NetPacket::new();
        pkt.init(true);
        pkt
    }

    /// OPTIONAL METHODS:
    //#[inline(always)]
    fn reset(&self, pkt: &mut NetPacket) {
        pkt.release();
    }

    ///
    #[inline(always)]
    fn is_valid(&self, _pkt: &NetPacket) -> bool {
        // you can optionally is_valid if object is good to be pushed back to the pool
        true
    }
}

///
//#[inline(always)]
pub fn take_packet(size: usize, leading_filed_size: u8) -> NetPacketGuard {
    if size <= SMALL_PACKET_MAX_SIZE {
        take_small_packet(leading_filed_size)
    } else {
        take_large_packet(leading_filed_size, size, b"")
    }
}

/// 申请 small packet
//#[inline(always)]
pub fn take_small_packet(leading_filed_size: u8) -> NetPacketGuard {
    let mut pkt = G_PACKET_POOL_SMALL.get();
    pkt.set_size_type(PacketSizeType::Small);
    pkt.set_leading_field_size(leading_filed_size);
    pkt
}

/// 申请 large packet
//#[inline(always)]
pub fn take_large_packet(
    leading_filed_size: u8,
    ensure_bytes: usize,
    init_slice: &[u8],
) -> NetPacketGuard {
    let mut pkt = G_PACKET_POOL_LARGE.get();
    pkt.set_size_type(PacketSizeType::Large);
    pkt.set_leading_field_size(leading_filed_size);
    pkt.ensure_writable_bytes(std::cmp::max(LARGE_BUFFER_INITIAL_SIZE, ensure_bytes));
    pkt.append_slice(init_slice);
    pkt
}
