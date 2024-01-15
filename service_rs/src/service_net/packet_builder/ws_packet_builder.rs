use std::sync::Arc;

use net_packet::NetPacketGuard;
use net_packet::{take_large_packet, take_packet};

use crate::service_net::packet::get_leading_field_size;
use crate::service_net::{ConnId, TcpConn};
use crate::ServiceRs;

const MAX_PACKET_SIZE: usize = 1024 * 1024 * 20; // 20M

/// Packet read result
pub enum WsPacketResult {
    Ready(Vec<NetPacketGuard>), // pkt list
    Abort(String),
}

/// Reader state
enum WsPacketBuilderState {
    Null,                   // 空包
    Leading,                // 包体前导长度
    Expand(usize),          // 扩展包体缓冲区（pkt_full_len）
    Data(usize),            // 包体数据区（pkt_full_len）
    Complete,               // 完成一个 pkt
    CompleteTailing(usize), // 完成一个 pkt，尾部冗余（pkt_full_len），申请新 pkt 容纳尾部多余数据
    Abort(usize),           // 中止（pkt_full_len）
}

///
pub struct WsPacketBuilder {
    state: WsPacketBuilderState,

    //
    pkt_opt: Option<NetPacketGuard>, // 使用 option 以便 pkt 移交
    build_cb: Box<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>,
}

impl WsPacketBuilder {
    ///
    pub fn new(build_cb: Box<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>) -> Self {
        Self {
            state: WsPacketBuilderState::Null,
            pkt_opt: None,
            build_cb,
        }
    }

    /// 解析数据包，触发数据包回调函数
    #[inline(always)]
    pub fn build(&mut self, conn: &Arc<TcpConn>, mut input_buffer: NetPacketGuard) {
        // 运行于 srv_net 线程
        assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());

        let leading_field_size = get_leading_field_size(conn.packet_type());
        input_buffer.set_leading_field_size(leading_field_size);

        //
        match self.build_once(conn.hd, input_buffer) {
            WsPacketResult::Ready(pkt_list) => {
                for pkt in pkt_list {
                    // trigger build_cb
                    (self.build_cb)(conn.clone(), pkt);
                }
            }

            WsPacketResult::Abort(err) => {
                //
                log::error!("[hd={}] build packet failed!!! error: {}", conn.hd, err);

                // low level close
                conn.close();
            }
        }
    }

    /* input_buffer 中存放 input 数据，一次性处理完毕，Ok 返回 pkt_list, Err 返回错误信息 */
    #[inline(always)]
    fn build_once(&mut self, _hd: ConnId, input_buffer: NetPacketGuard) -> WsPacketResult {
        //
        let mut pkt_list = Vec::new();

        let input_len = input_buffer.buffer_raw_len();
        let mut consumed = 0_usize;
        let mut remain = input_len;

        let leading_field_size = input_buffer.leading_field_size();

        // debug only
        /*{
            let input = input_buffer.peek();
            log::info!("[hd={}] input: ({}){:?}", hd, input.len(), input);
            let input_hex = hex::encode(input);
            log::info!("[hd={}] input_hex: ({}) --> {}", hd, input.len(), input_hex);
        }*/

        // input buffer 在循环中可能被 move，编译器会因为后续还有使用而报错，因此采用 option trick 封装一下
        let mut in_buf_opt = Some(input_buffer);

        //
        loop {
            //
            match self.state {
                WsPacketBuilderState::Null => {
                    let in_buf = in_buf_opt.take().unwrap();
                    let append_bytes = in_buf.buffer_raw_len();

                    // 初始化 pkt，直接使用 in_buf 避免 copy
                    self.pkt_opt = Some(in_buf);

                    // state: 进入包体前导长度读取
                    self.state = WsPacketBuilderState::Leading;

                    //
                    remain -= append_bytes;
                    consumed += append_bytes;
                }

                WsPacketBuilderState::Leading => {
                    let pkt = self.pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体前导长度字段是否完整？
                    if buffer_raw_len >= leading_field_size as usize {
                        // 查看取包体前导长度
                        let pkt_full_len = pkt.peek_leading_field();
                        if pkt_full_len > MAX_PACKET_SIZE {
                            // state: 中止
                            self.state = WsPacketBuilderState::Abort(pkt_full_len);
                        } else {
                            // 检查 pkt 容量是否足够
                            let writable_bytes = pkt.free_space();
                            if writable_bytes < pkt_full_len {
                                // state: 进入扩展包体缓冲区处理，重新申请 large pkt
                                self.state = WsPacketBuilderState::Expand(pkt_full_len);
                            } else {
                                // state: 进入包体数据处理
                                self.state = WsPacketBuilderState::Data(pkt_full_len);
                            }
                        }
                    } else if remain > 0 {
                        // in_buf 中还有 input 数据未处理，将此 input 数据附加到内部 pkt
                        let need_bytes = leading_field_size as usize - buffer_raw_len;
                        let append_bytes = if remain >= need_bytes {
                            need_bytes
                        } else {
                            remain
                        };

                        // 消耗 in_buf 中的 input 数据
                        let mut in_buf = in_buf_opt.take().unwrap();
                        let to_append = in_buf.consume_n(append_bytes);
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

                WsPacketBuilderState::Expand(pkt_full_len) => {
                    let old_pkt = self.pkt_opt.as_mut().unwrap();

                    //
                    let ensure_bytes = pkt_full_len;
                    let old_pkt_slice = old_pkt.consume();

                    // old pkt 数据转移到 new pkt，使用 new pkt 继续解析
                    let mut new_pkt = take_large_packet(ensure_bytes);
                    new_pkt.append_slice(old_pkt_slice);
                    new_pkt.set_leading_field_size(leading_field_size);
                    self.pkt_opt = Some(new_pkt);

                    // 进入包体数据处理
                    self.state = WsPacketBuilderState::Data(pkt_full_len);
                }

                WsPacketBuilderState::Data(pkt_full_len) => {
                    let pkt = self.pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体数据是否完整？
                    if buffer_raw_len > pkt_full_len {
                        // state: 完成当前 pkt 读取，转入完成处理（pkt尾部有多余数据）
                        self.state = WsPacketBuilderState::CompleteTailing(pkt_full_len);
                    } else if buffer_raw_len == pkt_full_len {
                        // state: 完成当前 pkt 读取，转入完成处理
                        self.state = WsPacketBuilderState::Complete;
                    } else if remain > 0 {
                        // in_buf 中还有 input 数据未处理，将此 input 数据附加到内部 pkt
                        let need_types = pkt_full_len - buffer_raw_len;
                        let append_bytes = if remain >= need_types {
                            need_types
                        } else {
                            remain
                        };

                        // 消耗 in_buf 中的 input 数据
                        let in_buf = in_buf_opt.as_mut().unwrap();
                        let to_append = in_buf.consume_n(append_bytes);
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

                WsPacketBuilderState::Complete => {
                    // 完成一个 pkt
                    let ready_pkt = self.pkt_opt.take().unwrap();
                    self.pkt_opt = None;

                    //
                    pkt_list.push(ready_pkt);

                    // 完成当前 pkt 读取，进入空包状态
                    self.state = WsPacketBuilderState::Null;

                    // input 数据处理完毕
                    if 0 == remain {
                        break;
                    }
                    assert!(in_buf_opt.is_some()); // 还有残余数据在 in_buf 中
                }

                WsPacketBuilderState::CompleteTailing(pkt_full_len) => {
                    // 截取多余的尾部，创建新包
                    let pkt = self.pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();
                    assert!(pkt_full_len < buffer_raw_len);

                    let tail_len = buffer_raw_len - pkt_full_len;

                    // 复制字节数较少的部分
                    if pkt_full_len < tail_len {
                        // 完成包较小，使用新包进行容纳
                        let mut ready_pkt = take_packet(pkt_full_len);
                        ready_pkt.set_leading_field_size(leading_field_size);

                        //
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
                            let mut tail_pkt = take_packet(pkt_full_len);
                            tail_pkt.set_leading_field_size(leading_field_size);
                            tail_pkt.append_slice(to_append);

                            self.pkt_opt = Some(tail_pkt);
                        }

                        //
                        pkt_list.push(ready_pkt);
                    }

                    // 完成当前 pkt 读取，重新开始包体前导长度处理
                    self.state = WsPacketBuilderState::Leading;
                }

                WsPacketBuilderState::Abort(pkt_full_len) => {
                    log::error!("packet overflow!!! pkt_full_len={}", pkt_full_len);

                    // 包长度越界
                    return WsPacketResult::Abort("overflow".to_owned());
                }
            }
        }

        // 完成包列表
        assert_eq!(consumed, input_len);
        WsPacketResult::Ready(pkt_list)
    }
}
