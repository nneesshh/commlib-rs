use net_packet::Buffer;

use crate::service_net::redis::reply_builder::{BuildResult, ReplySubBuilder};
use crate::{RedisReply, RedisReplyType};

use super::{IntegerBuilder, RootBuilder};

#[derive(Debug)]
enum ArrayBuildState {
    Size,
    Field(i64), // field with item num
}

/// 构造 Array
pub struct ArrayBuilder {
    integer_builder: IntegerBuilder,
    root_builder: RootBuilder, // use Box to avoid recursive struct
    build_state: ArrayBuildState,

    //
    reply_list: Vec<RedisReply>,
}

impl ArrayBuilder {
    ///
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            integer_builder: IntegerBuilder(),
            root_builder: RootBuilder::new(),
            build_state: ArrayBuildState::Size,

            reply_list: Vec::new(),
        }
    }

    ///
    #[inline(always)]
    pub fn reset(&mut self) {
        self.build_state = ArrayBuildState::Size;
        self.reply_list.clear();
    }
}

impl ReplySubBuilder for ArrayBuilder {
    ///
    #[inline(always)]
    fn try_build(&mut self, buffer: &mut Buffer) -> BuildResult {
        loop {
            match self.build_state {
                ArrayBuildState::Size => {
                    //
                    match self.integer_builder.try_build(buffer) {
                        BuildResult::Success(reply) => {
                            self.build_state = ArrayBuildState::Field(reply.as_integer());
                        }
                        BuildResult::Suspend => {
                            return BuildResult::Suspend;
                        }
                        BuildResult::ErrorInvalidInteger => {
                            return BuildResult::ErrorInvalidInteger;
                        }
                    }
                }
                ArrayBuildState::Field(num) => {
                    if 0_i64 == num {
                        // "*0\r\n
                        let r = RedisReply::from("", RedisReplyType::BulkString);

                        buffer.advance(2);
                        self.reset();
                        return BuildResult::Success(r);
                    } else if -1_i64 == num {
                        // "$-1\r\n" means "nil"
                        let r = RedisReply::null(); // null

                        self.reset();
                        return BuildResult::Success(r);
                    } else {
                        assert!(num > 0_i64);
                        while self.reply_list.len() < num as usize {
                            //
                            match self.root_builder.try_build(buffer) {
                                BuildResult::Success(reply) => {
                                    //
                                    self.reply_list.push(reply);
                                }
                                BuildResult::Suspend => {
                                    // not enough input
                                    return BuildResult::Suspend;
                                }
                                BuildResult::ErrorInvalidInteger => {
                                    // state: Abort
                                    log::error!(
                                        "[ArrayBuilder] try_build failed!!! ErrorInvalidInteger!!!"
                                    );
                                    return BuildResult::ErrorInvalidInteger;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
