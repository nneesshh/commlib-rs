use std::sync::Arc;

use crate::service_net::tcp_conn_manager::on_connection_closed;
use crate::RedisReply;
use crate::{Buffer, NetPacketGuard, ServiceRs, TcpConn};

use super::sub_builder_impl::RootBuilder;

const REDIS_BUFFER_INITIAL_SIZE: usize = 4096;
const REDIS_BUFFER_RESERVED_PREPEND_SIZE: usize = 0;

/// Reply read result
pub enum ReplyResult {
    Ready(Vec<RedisReply>), // reply list
    Abort(String),
}

///
pub enum BuildResult {
    Success(RedisReply),
    Suspend,
    ErrorInvalidInteger, // 无效的整数类型
}

/// Build reply from buffer
pub trait ReplySubBuilder {
    ///
    fn try_build(&mut self, buffer: &mut Buffer) -> BuildResult;
}

///
pub struct ReplyBuilder {
    //
    reply_fn: Arc<dyn Fn(Arc<TcpConn>, RedisReply) + Send + Sync>,

    //
    buffer: Buffer,

    //
    root_builder: RootBuilder,
}

impl ReplyBuilder {
    ///
    pub fn new(reply_fn: Arc<dyn Fn(Arc<TcpConn>, RedisReply) + Send + Sync>) -> Self {
        Self {
            reply_fn,

            buffer: Buffer::new(
                REDIS_BUFFER_INITIAL_SIZE,
                REDIS_BUFFER_RESERVED_PREPEND_SIZE,
            ),

            root_builder: RootBuilder::new(),
        }
    }

    /// 解析 RedisReply，触发回调函数
    #[inline(always)]
    pub fn build(&self, conn: &Arc<TcpConn>, input_buffer: NetPacketGuard) {
        // 运行于 srv_net 线程
        assert!(conn.srv_net.is_in_service_thread());

        let builder = unsafe { &mut *(self as *const Self as *mut Self) };

        //
        match builder.build_once(input_buffer) {
            ReplyResult::Ready(reply_list) => {
                for reply in reply_list {
                    // trigger reply_fn
                    let conn = conn.clone();
                    let f = self.reply_fn.clone();
                    let srv = conn.srv.clone();

                    srv.run_in_service(Box::new(move || {
                        (f)(conn, reply);
                    }));
                }
            }

            ReplyResult::Abort(err) => {
                //
                log::error!("[hd={}] build reply failed!!! error: {}", conn.hd, err);

                // low level close
                conn.close();

                // trigger connetion closed event
                on_connection_closed(&conn.srv_net, conn.hd);
            }
        }
    }

    /* input_buffer 中存放 input 数据，一次性处理完毕，Ok 返回 reply_list, Err 返回错误信息 */
    #[inline(always)]
    fn build_once(&mut self, input_buffer: NetPacketGuard) -> ReplyResult {
        //
        let mut reply_list: Vec<RedisReply> = Vec::new();

        // debug only
        /*{
            let input = input_buffer.peek();
            log::info!("input: {:?}", input);
            let input_hex = hex::encode(input);
            log::info!("input_hex: {}", input_hex);
        }*/

        self.buffer.write_slice(input_buffer.peek());

        //
        loop {
            match self.root_builder.try_build(&mut self.buffer) {
                BuildResult::Success(reply) => {
                    //
                    reply_list.push(reply);
                }
                BuildResult::Suspend => {
                    // not enough input
                    break;
                }
                BuildResult::ErrorInvalidInteger => {
                    // state: Abort
                    log::error!("[ReplyBuilder] build_once failed!!! ErrorInvalidInteger!!!");
                    return ReplyResult::Abort("ErrorInvalidInteger".to_owned());
                }
            }
        }

        // 完成包列表
        ReplyResult::Ready(reply_list)
    }
}
