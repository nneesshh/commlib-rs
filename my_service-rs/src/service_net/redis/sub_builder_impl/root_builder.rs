use net_packet::Buffer;

use crate::service_net::redis::reply_builder::{BuildResult, ReplySubBuilder};

use super::{ArrayBuilder, BulkStringBuilder, ErrorBuilder, IntegerBuilder, SimpleStringBuilder};

#[derive(Debug)]
enum RootBuildState {
    Leading,             // 包体前导长度
    SimpleStringBuilder, // '+'
    ErrorBuilder,        // '-'
    IntegerBuilder,      // ':'
    BulkStringBuilder,   // '$'
    ArrayBuilder,        // '*'
    Abort,               // 中止(reason)
}

/// 构造 Root
pub struct RootBuilder {
    //
    state: RootBuildState,

    // sub builders
    simple_string_builder: SimpleStringBuilder, // '+'
    error_builder: ErrorBuilder,                // '-'
    integer_builder: IntegerBuilder,            // ':'
    bulk_string_builder: BulkStringBuilder,     // '$'
    array_builder: Option<Box<ArrayBuilder>>,   // '*'
}

impl RootBuilder {
    ///
    pub fn new() -> Self {
        Self {
            state: RootBuildState::Leading,

            simple_string_builder: SimpleStringBuilder(),
            error_builder: ErrorBuilder::new(),
            integer_builder: IntegerBuilder(),
            bulk_string_builder: BulkStringBuilder::new(),
            array_builder: None,
        }
    }
}

impl ReplySubBuilder for RootBuilder {
    ///
    #[inline(always)]
    fn try_build(&mut self, mut buffer: &mut Buffer) -> BuildResult {
        loop {
            //
            match &self.state {
                RootBuildState::Leading => {
                    let buffer_raw_len = buffer.size();

                    // 包体前导长度字段是否完整？
                    if buffer_raw_len >= 1_usize {
                        // 查看取包体前导长度
                        let builder_flag = buffer.read_u8() as char;
                        match builder_flag {
                            '+' => {
                                // state: simple string
                                self.state = RootBuildState::SimpleStringBuilder;
                            }
                            '-' => {
                                // state: error
                                self.state = RootBuildState::ErrorBuilder;
                            }
                            ':' => {
                                // state: integer
                                self.state = RootBuildState::IntegerBuilder;
                            }
                            '$' => {
                                // state: bulk string
                                self.state = RootBuildState::BulkStringBuilder;
                            }
                            '*' => {
                                // state: array
                                self.state = RootBuildState::ArrayBuilder;

                                // create array builder when needed
                                if self.array_builder.is_none() {
                                    self.array_builder = Some(Box::new(ArrayBuilder::new()));
                                }
                            }
                            _ => {
                                // state: 中止
                                self.state = RootBuildState::Abort;
                                return BuildResult::ErrorInvalidInteger;
                            }
                        }
                    } else {
                        // not enough input
                        return BuildResult::Suspend;
                    }
                }

                RootBuildState::SimpleStringBuilder => {
                    //
                    match self.simple_string_builder.try_build(&mut buffer) {
                        BuildResult::Success(reply) => {
                            // state: 重新开始
                            self.state = RootBuildState::Leading;

                            //
                            return BuildResult::Success(reply);
                        }
                        BuildResult::Suspend => {
                            // not enough input
                            return BuildResult::Suspend;
                        }
                        BuildResult::ErrorInvalidInteger => {
                            // state: 中止
                            self.state = RootBuildState::Abort;
                            return BuildResult::ErrorInvalidInteger;
                        }
                    }
                }

                RootBuildState::ErrorBuilder => {
                    //
                    match self.error_builder.try_build(&mut buffer) {
                        BuildResult::Success(reply) => {
                            // state: 重新开始
                            self.state = RootBuildState::Leading;

                            //
                            return BuildResult::Success(reply);
                        }
                        BuildResult::Suspend => {
                            // not enough input
                            return BuildResult::Suspend;
                        }
                        BuildResult::ErrorInvalidInteger => {
                            // state: 中止
                            self.state = RootBuildState::Abort;
                            return BuildResult::ErrorInvalidInteger;
                        }
                    }
                }

                RootBuildState::IntegerBuilder => {
                    //
                    match self.integer_builder.try_build(&mut buffer) {
                        BuildResult::Success(reply) => {
                            // state: 重新开始
                            self.state = RootBuildState::Leading;

                            //
                            return BuildResult::Success(reply);
                        }
                        BuildResult::Suspend => {
                            // not enough input
                            return BuildResult::Suspend;
                        }
                        BuildResult::ErrorInvalidInteger => {
                            // state: 中止
                            self.state = RootBuildState::Abort;
                            return BuildResult::ErrorInvalidInteger;
                        }
                    }
                }

                RootBuildState::BulkStringBuilder => {
                    //
                    match self.bulk_string_builder.try_build(&mut buffer) {
                        BuildResult::Success(reply) => {
                            // state: 重新开始
                            self.state = RootBuildState::Leading;

                            //
                            return BuildResult::Success(reply);
                        }
                        BuildResult::Suspend => {
                            // not enough input
                            return BuildResult::Suspend;
                        }
                        BuildResult::ErrorInvalidInteger => {
                            // state: 中止
                            self.state = RootBuildState::Abort;
                            return BuildResult::ErrorInvalidInteger;
                        }
                    }
                }

                RootBuildState::ArrayBuilder => {
                    //
                    let array_builder = self.array_builder.as_mut().unwrap();
                    match array_builder.try_build(&mut buffer) {
                        BuildResult::Success(reply) => {
                            // state: 重新开始
                            self.state = RootBuildState::Leading;

                            //
                            return BuildResult::Success(reply);
                        }
                        BuildResult::Suspend => {
                            // not enough input
                            return BuildResult::Suspend;
                        }
                        BuildResult::ErrorInvalidInteger => {
                            // state: 中止
                            self.state = RootBuildState::Abort;
                            return BuildResult::ErrorInvalidInteger;
                        }
                    }
                }

                RootBuildState::Abort => {
                    // IT SHOULD NEVER HAPPEN
                    log::error!("root builder abort!!!");
                    std::unreachable!()
                }
            }
        }
    }
}
