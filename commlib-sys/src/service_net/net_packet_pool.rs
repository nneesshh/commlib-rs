use super::net_packet::NetPacket;
use lazy_static::lazy_static;
use opool::{Pool, PoolAllocator, RefGuard};

///
pub struct NetPacketPool;

const SMALL_PACKEG_MAX_SIZE: usize = 1024 * 4;
impl PoolAllocator<NetPacket> for NetPacketPool {
    #[inline]
    fn allocate(&self) -> NetPacket {
        NetPacket::new()
    }

    /// OPTIONAL METHODS:
    #[inline]
    fn reset(&self, _obj: &mut NetPacket) {
        // Optionally you can clear or zero object fields here
    }

    #[inline]
    fn is_valid(&self, _obj: &NetPacket) -> bool {
        // you can optionally is_valid if object is good to be pushed back to the pool
        true
    }
}

///
pub fn take_packet(size: usize) -> RefGuard<'static, NetPacketPool, NetPacket> {
    if size < SMALL_PACKEG_MAX_SIZE {
        G_PACKET_POOL_SMALL.get()
    } else {
        G_PACKET_POOL_LARGE.get()
    }
}

lazy_static! {
    // < 4k (SMALL_PACKEG_MAX_SIZE), 最多 cache 8192个
    pub static ref G_PACKET_POOL_LARGE: Pool<NetPacketPool, NetPacket> = Pool::new(8192, NetPacketPool);

    // >= 4k (SMALL_PACKEG_MAX_SIZE), 最多 cache 128个, 超过数量上限立即释放，避免占用过多内存
    pub static ref G_PACKET_POOL_SMALL: Pool<NetPacketPool, NetPacket> = Pool::new(128, NetPacketPool);
}
