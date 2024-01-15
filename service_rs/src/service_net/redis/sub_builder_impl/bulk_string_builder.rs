use net_packet::Buffer;

use crate::service_net::redis::reply_builder::{BuildResult, ReplySubBuilder};
use crate::{RedisReply, RedisReplyType};

use super::IntegerBuilder;

#[derive(Debug)]
enum BulkStringBuildState {
    Size,
    Field(i64), // field with length
}

/// 构造 Bulk 字符串
#[derive(Debug)]
pub struct BulkStringBuilder {
    integer_builder: IntegerBuilder,
    build_state: BulkStringBuildState,
}

impl BulkStringBuilder {
    ///
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            integer_builder: IntegerBuilder(),
            build_state: BulkStringBuildState::Size,
        }
    }
}

impl ReplySubBuilder for BulkStringBuilder {
    ///
    #[inline(always)]
    fn try_build(&mut self, buffer: &mut Buffer) -> BuildResult {
        loop {
            match self.build_state {
                BulkStringBuildState::Size => {
                    //
                    match self.integer_builder.try_build(buffer) {
                        BuildResult::Success(reply) => {
                            self.build_state = BulkStringBuildState::Field(reply.as_integer());
                        }
                        BuildResult::Suspend => {
                            return BuildResult::Suspend;
                        }
                        BuildResult::ErrorInvalidInteger => {
                            return BuildResult::ErrorInvalidInteger;
                        }
                    }
                }
                BulkStringBuildState::Field(len) => {
                    if 0_i64 == len {
                        // "$0\r\n\r\n
                        let r = RedisReply::from("", RedisReplyType::BulkString);
                        buffer.advance(2);

                        self.build_state = BulkStringBuildState::Size;
                        return BuildResult::Success(r);
                    } else if -1_i64 == len {
                        // "$-1\r\n" means "nil"
                        let r = RedisReply::null(); // null

                        self.build_state = BulkStringBuildState::Size;
                        return BuildResult::Success(r);
                    } else {
                        let s = unsafe { std::str::from_utf8_unchecked(buffer.peek()) };
                        let pat: &[_] = &['\r', '\n'];
                        if let Some(pos) = s.find(pat) {
                            let value = &s[..pos];
                            let r = RedisReply::from(value, RedisReplyType::BulkString);
                            buffer.advance(pos + 2); // 2 is "\r\n" length

                            self.build_state = BulkStringBuildState::Size;
                            return BuildResult::Success(r);
                        } else {
                            return BuildResult::Suspend;
                        }
                    }
                }
            }
        }
    }
}
