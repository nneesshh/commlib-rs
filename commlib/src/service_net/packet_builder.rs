use std::sync::Arc;

use crate::ServiceRs;

use super::tcp_conn_manager::on_connection_closed;
use super::NetPacketGuard;
use super::TcpConn;
use super::{get_leading_field_size, take_large_packet, take_packet};

const MAX_PACKET_SIZE: usize = 1024 * 1024 * 20; // 20M

/// Packet read result
pub enum PacketResult {
    Ready(Vec<NetPacketGuard>), // pkt list
    Abort(String),
}

/// Reader state
enum PacketBuilderState {
    Null,                   // 空包
    Leading,                // 包体前导长度
    Expand(usize),          // 扩展包体缓冲区（pkt_full_len）
    Data(usize),            // 包体数据区（pkt_full_len）
    Complete,               // 完成一个 pkt
    CompleteTailing(usize), // 完成一个 pkt，尾部冗余（pkt_full_len），申请新 pkt 容纳尾部多余数据
    Abort(usize),           // 中止（pkt_full_len）
}

///
pub struct PacketBuilder {
    state: PacketBuilderState,

    //
    pkt_opt: Option<NetPacketGuard>, // 使用 option 以便 pkt 移交
    pkt_fn: Arc<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>,
}

impl PacketBuilder {
    ///
    pub fn new(pkt_fn: Arc<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>) -> Self {
        Self {
            state: PacketBuilderState::Null,
            pkt_opt: None,
            pkt_fn,
        }
    }

    /// 解析数据包，触发数据包回调函数
    #[inline(always)]
    pub fn build(&self, conn: &Arc<TcpConn>, mut input_buffer: NetPacketGuard) {
        // 运行于 srv_net 线程
        assert!(conn.srv_net.is_in_service_thread());

        let builder = unsafe { &mut *(self as *const Self as *mut Self) };

        let leading_field_size = get_leading_field_size(conn.packet_type());
        input_buffer.set_leading_field_size(leading_field_size);

        //
        match builder.build_once(input_buffer) {
            PacketResult::Ready(pkt_list) => {
                for pkt in pkt_list {
                    // trigger pkt_fn
                    let conn = conn.clone();
                    let f = self.pkt_fn.clone();
                    let srv = conn.srv.clone();

                    srv.run_in_service(Box::new(move || {
                        (f)(conn, pkt);
                    }));
                }
            }

            PacketResult::Abort(err) => {
                //
                log::error!("[hd={}] build packet failed!!! error: {}", conn.hd, err);

                // low level close
                conn.close();

                // trigger connetion closed event
                on_connection_closed(&conn.srv_net, conn.hd);
            }
        }
    }

    /* input_buffer 中存放 input 数据，一次性处理完毕，Ok 返回 pkt_list, Err 返回错误信息 */
    #[inline(always)]
    fn build_once(&mut self, input_buffer: NetPacketGuard) -> PacketResult {
        //
        let mut pkt_list = Vec::new();

        let mut consumed = 0_usize;
        let mut remain = input_buffer.buffer_raw_len();

        let leading_field_size = input_buffer.leading_field_size();

        // debug only
        /*{
            let input = input_buffer.peek();
            log::info!("input: ({}){:?}", input.len(), input);
            let input_hex = hex::encode(input);
            log::info!("input_hex: {}", input_hex);
        }*/

        // input buffer 在循环中可能被 move，编译器会因为后续还有使用而报错，因此采用 option trick 封装一下
        let mut input_pkt_opt = Some(input_buffer);

        //
        loop {
            //
            match self.state {
                PacketBuilderState::Null => {
                    let input_pkt = input_pkt_opt.take().unwrap();
                    let append_bytes = input_pkt.buffer_raw_len();

                    // 初始化 pkt，直接使用 input_pkt 避免 copy
                    self.pkt_opt = Some(input_pkt);

                    // state: 进入包体前导长度读取
                    self.state = PacketBuilderState::Leading;

                    //
                    remain -= append_bytes;
                    consumed += append_bytes;
                }

                PacketBuilderState::Leading => {
                    let pkt = self.pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体前导长度字段是否完整？
                    if buffer_raw_len >= leading_field_size as usize {
                        // 查看取包体前导长度
                        let pkt_full_len = pkt.peek_leading_field();
                        if pkt_full_len > MAX_PACKET_SIZE {
                            // state: 中止
                            self.state = PacketBuilderState::Abort(pkt_full_len);
                        } else {
                            // 检查 pkt 容量是否足够
                            let writable_bytes = pkt.buffer_writable_bytes();
                            if writable_bytes < pkt_full_len {
                                // state: 进入扩展包体缓冲区处理，重新申请 large pkt
                                self.state = PacketBuilderState::Expand(pkt_full_len);
                            } else {
                                // state: 进入包体数据处理
                                self.state = PacketBuilderState::Data(pkt_full_len);
                            }
                        }
                    } else if remain > 0 {
                        // input_buffer 中还有 input 数据未处理，将此 input 数据附加到内部 pkt
                        let need_bytes = leading_field_size as usize - buffer_raw_len;
                        let append_bytes = if remain >= need_bytes {
                            need_bytes
                        } else {
                            remain
                        };

                        // 消耗 input_buffer 中的 input 数据
                        let mut input_pkt = input_pkt_opt.take().unwrap();
                        let to_append = input_pkt.consume_n(append_bytes);
                        pkt.append_slice(to_append);

                        //
                        remain -= append_bytes;
                        consumed += append_bytes;

                        // not enough for append (输入数据已经消耗完毕)
                        if append_bytes < need_bytes {
                            break;
                        }
                    } else {
                        // not enough for append (输入数据已经消耗完毕)
                        break;
                    }
                }

                PacketBuilderState::Expand(pkt_full_len) => {
                    let old_pkt = self.pkt_opt.as_mut().unwrap();

                    //
                    let ensure_bytes = pkt_full_len;
                    let old_pkt_slice = old_pkt.consume();

                    // old pkt 数据转移到 new pkt，使用 new pkt 继续解析
                    let new_pkt =
                        take_large_packet(leading_field_size, ensure_bytes, old_pkt_slice);
                    self.pkt_opt = Some(new_pkt);

                    // 进入包体数据处理
                    self.state = PacketBuilderState::Data(pkt_full_len);
                }

                PacketBuilderState::Data(pkt_full_len) => {
                    let pkt = self.pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体数据是否完整？
                    if buffer_raw_len > pkt_full_len {
                        // state: 完成当前 pkt 读取，转入完成处理（pkt尾部有多余数据）
                        self.state = PacketBuilderState::CompleteTailing(pkt_full_len);
                    } else if buffer_raw_len == pkt_full_len {
                        // state: 完成当前 pkt 读取，转入完成处理
                        self.state = PacketBuilderState::Complete;
                    } else if remain > 0 {
                        // input_buffer 中还有 input 数据未处理，将此 input 数据附加到内部 pkt
                        assert!(buffer_raw_len + consumed <= pkt_full_len);
                        let need_types = pkt_full_len - (buffer_raw_len + consumed);
                        let append_bytes = if remain >= need_types {
                            need_types
                        } else {
                            remain
                        };

                        // 消耗 input_pkt 中的 input 数据
                        let input_pkt = input_pkt_opt.as_mut().unwrap();
                        let to_append = input_pkt.consume_n(append_bytes);
                        pkt.append_slice(to_append);

                        //
                        remain -= append_bytes;
                        consumed += append_bytes;

                        // not enough for append (输入数据已经消耗完毕)
                        if append_bytes < need_types {
                            break;
                        }
                    } else {
                        // not enough for append (输入数据已经消耗完毕)
                        break;
                    }
                }

                PacketBuilderState::Complete => {
                    // 完成一个 pkt
                    let ready_pkt = self.pkt_opt.take().unwrap();
                    self.pkt_opt = None;

                    //
                    pkt_list.push(ready_pkt);

                    // 完成当前 pkt 读取，进入空包状态
                    self.state = PacketBuilderState::Null;

                    // input 数据处理完毕
                    if 0 == remain {
                        break;
                    }
                    assert!(input_pkt_opt.is_some()); // 还有残余数据在 input buffer 中
                }

                PacketBuilderState::CompleteTailing(pkt_full_len) => {
                    // 截取多余的尾部，创建新包
                    let pkt = self.pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();
                    assert!(pkt_full_len < buffer_raw_len);

                    let tail_len = buffer_raw_len - pkt_full_len;

                    // 复制字节数较少的部分
                    if pkt_full_len < tail_len {
                        // 完成包较小，使用新包进行容纳
                        let mut ready_pkt = take_packet(pkt_full_len, leading_field_size);
                        {
                            let to_append = pkt.consume_n(pkt_full_len);
                            ready_pkt.append_slice(to_append);
                        }

                        //
                        pkt_list.push(ready_pkt);
                    } else {
                        // 尾部较小，使用新包进行容纳
                        let mut ready_pkt = self.pkt_opt.take().unwrap();
                        {
                            let to_append = ready_pkt.consume_tail_n(tail_len);

                            //
                            let mut tail_pkt = take_packet(pkt_full_len, leading_field_size);
                            tail_pkt.append_slice(to_append);

                            self.pkt_opt = Some(tail_pkt);
                        }

                        //
                        pkt_list.push(ready_pkt);
                    }

                    // 完成当前 pkt 读取，重新开始包体前导长度处理
                    self.state = PacketBuilderState::Leading;
                }

                PacketBuilderState::Abort(pkt_full_len) => {
                    log::error!("packet overflow!!! pkt_full_len={}", pkt_full_len);

                    // 包长度越界
                    return PacketResult::Abort("overflow".to_owned());
                }
            }
        }

        // 完成包列表
        PacketResult::Ready(pkt_list)
    }
}
