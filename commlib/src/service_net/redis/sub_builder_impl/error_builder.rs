use net_packet::Buffer;

use crate::service_net::redis::reply_builder::{BuildResult, ReplySubBuilder};
use crate::RedisReply;

use super::SimpleStringBuilder;

/// 构造 Error
#[derive(Debug)]
pub struct ErrorBuilder {
    simple_string_builder: SimpleStringBuilder,
}

impl ErrorBuilder {
    ///
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            simple_string_builder: SimpleStringBuilder(),
        }
    }
}

impl ReplySubBuilder for ErrorBuilder {
    ///
    #[inline(always)]
    fn try_build(&mut self, buffer: &mut Buffer) -> BuildResult {
        //
        match self.simple_string_builder.try_build(buffer) {
            BuildResult::Success(reply) => {
                let r = RedisReply::from_error(reply.as_string());
                BuildResult::Success(r)
            }
            BuildResult::Suspend => BuildResult::Suspend,
            BuildResult::ErrorInvalidInteger => {
                std::unreachable!()
            }
        }
    }
}
