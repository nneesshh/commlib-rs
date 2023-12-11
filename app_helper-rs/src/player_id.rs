//! Commlib: PlayerId
//!
//! PID 编码的坑：
//! lua5.1 整数精度最大 52bit，但其实超过 46bit (46bit 转换为十进制为 14 个十进制数字，47bit 为 15 个十进制数字，就会
//! 超过 10e14大小) 后，数字就会开始失真：例如低位数字丢失精度。对于 zone 超过 15bit (上限 32768) 的区服， openresty
//! 必须使用字符串来传递 pid 的值
//! +----------------+----------------+
//! |     zone       |    sequence    |
//! +----------------+----------------+
//! |     15bit      |      31bit     |
//! +----------------+----------------+
//! |    1~32,768    |1~2,147,487,648 |
//! +----------------+----------------+

use commlib::{PlayerId, ZoneId};

///
pub const PID_UID_BITS: i32 = 31_i32;

///
#[inline(always)]
pub fn make_player_id(zone: ZoneId, uid: u32) -> PlayerId {
    ((zone as PlayerId) << PID_UID_BITS) + uid as PlayerId
}

///
#[inline(always)]
pub fn zone_from_pid(pid: PlayerId) -> ZoneId {
    (pid >> PID_UID_BITS) as ZoneId
}
