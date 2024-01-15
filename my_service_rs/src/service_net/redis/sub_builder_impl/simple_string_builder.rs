use net_packet::Buffer;

use crate::service_net::redis::reply_builder::{BuildResult, ReplySubBuilder};
use crate::{RedisReply, RedisReplyType};

/// 构造简单字符串
#[derive(Debug)]
pub struct SimpleStringBuilder();

impl ReplySubBuilder for SimpleStringBuilder {
    ///
    #[inline(always)]
    fn try_build(&mut self, buffer: &mut Buffer) -> BuildResult {
        let s = unsafe { std::str::from_utf8_unchecked(buffer.peek()) };
        let pat: &[_] = &['\r', '\n'];
        if let Some(pos) = s.find(pat) {
            // "+abcdefg\r\n"
            let value = &s[..pos];
            let r = RedisReply::from(value, RedisReplyType::SimpleString);
            buffer.advance(pos + 2); // 2 is "\r\n" length
            BuildResult::Success(r)
        } else {
            BuildResult::Suspend
        }
    }
}
