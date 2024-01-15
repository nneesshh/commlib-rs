use net_packet::Buffer;

use crate::service_net::redis::reply_builder::{BuildResult, ReplySubBuilder};
use crate::RedisReply;

/// 构造 Integer
#[derive(Debug)]
pub struct IntegerBuilder();

impl ReplySubBuilder for IntegerBuilder {
    ///
    #[inline(always)]
    fn try_build(&mut self, buffer: &mut Buffer) -> BuildResult {
        let s = unsafe { std::str::from_utf8_unchecked(buffer.peek()) };
        let pat: &[_] = &['\r', '\n'];
        if let Some(pos) = s.find(pat) {
            let value = &s[..pos];
            if let Ok(int_value) = value.parse::<i64>() {
                let r = RedisReply::from_integer(int_value);
                buffer.advance(pos + 2); // 2 is "\r\n" length
                BuildResult::Success(r)
            } else {
                BuildResult::ErrorInvalidInteger
            }
        } else {
            BuildResult::Suspend
        }
    }
}
