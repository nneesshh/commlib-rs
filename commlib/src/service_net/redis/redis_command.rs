use std::fmt::Write;
use std::sync::Arc;

use crate::pinky_swear::{Pinky, PinkySwear};
use crate::{Buffer, RedisReply, TcpConn};

const MAX_COMMANDS_PER_COMMIT: usize = 512;

const CMD_BUFFER_INITIAL_SIZE: usize = 4096;
const CMD_BUFFER_RESERVED_PREPEND_SIZE: usize = 0;

///
pub type ReplyCallback = Arc<dyn Fn(RedisReply) + Send + Sync>;

///
pub struct CommandResponse {
    reply: RedisReply,
    cb: ReplyCallback,
}

///
pub struct Command {
    cmd: Vec<String>,
    cb: ReplyCallback,
    pinky_opt: Option<Pinky<CommandResponse>>,
}

///
pub struct RedisCommand {
    //
    name: String,
    pass: String,   // redis 密码
    dbindex: isize, // redis db index

    //
    conn_opt: Option<Arc<TcpConn>>,
    commands: Vec<Command>,
    commands_bytes: usize,
    cb_running_num: usize,

    //
    buffer: Buffer,

    //
    ready: bool,
}

impl RedisCommand {
    ///
    pub fn new(name: &str, pass: &str, dbindex: isize) -> Self {
        Self {
            name: name.to_owned(),
            pass: pass.to_owned(),
            dbindex,

            conn_opt: None,
            commands: Vec::new(),
            commands_bytes: 0,
            cb_running_num: 0,

            buffer: Buffer::new(CMD_BUFFER_INITIAL_SIZE, CMD_BUFFER_RESERVED_PREPEND_SIZE),

            ready: false,
        }
    }

    /// 连接成功
    pub fn on_link_connected(&mut self, conn: &Arc<TcpConn>) {
        //
        log::debug!("redis[{}] on_link_connected", self.name);

        self.bind_conn(conn);

        self.do_auth();
        self.do_select();

        //
        self.commit();
        self.ready = true;
    }

    /// 收到 reply
    pub fn on_reply(&mut self, reply: RedisReply) {}

    ///
    pub fn on_link_broken(&mut self) {
        //
        log::debug!("redis[{}] on_link_broken", self.name);

        //
        self.ready = false;
        self.conn_opt = None;
    }

    ///
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    ///
    pub fn send<F>(&mut self, cmd: Vec<String>, cb: F, pinky_opt: Option<Pinky<CommandResponse>>)
    where
        F: Fn(RedisReply) + Send + Sync + 'static,
    {
        assert!(cmd.len() > 0);
        assert!(self.commands.len() < MAX_COMMANDS_PER_COMMIT);

        for part in &cmd {
            self.commands_bytes += part.len();
        }

        self.commands.push(Command {
            cmd,
            cb: Arc::new(cb),
            pinky_opt,
        });

        self.cb_running_num += 1;
    }

    /// 异步提交： 如果提交失败，则在连接成功后再提交一次
    pub fn commit(&mut self) -> bool {
        //
        if let Some(conn) = self.conn_opt.as_ref() {
            let conn = conn.clone();
            if !conn.is_closed() {
                //
                {
                    self.build_commands();
                }

                //
                {
                    let data = self.buffer.next_all();
                    conn.send(data);
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// 同步提交： 如果提交失败，则在连接成功后再提交一次
    pub fn commit_blocking(&mut self) {}

    /// 用户手工调用，清除 commands 缓存
    pub fn clear_commands(&mut self) {
        //
        for command in &self.commands {
            let cmd_tips = &command.cmd[0];

            log::info!(
                "redis[{}] command:{} cleared, total={} running={}",
                self.name,
                cmd_tips,
                self.commands.len(),
                self.cb_running_num
            );

            //
            if let Some(pinky) = command.pinky_opt.as_ref() {
                // sim nil reply for promise
                let nil_reply = RedisReply::null();
                pinky.swear(CommandResponse {
                    reply: nil_reply,
                    cb: command.cb.clone(),
                });
            } else {
                // DO NOTHING: just drop the callback
                // DON'T callback in the current thread, it may not be the origin thread
            }
        }

        // 清空
        self.commands.clear();
    }

    /// 同步 auth
    pub fn do_auth(&mut self) {
        //
        let (prms, pinky) = PinkySwear::<CommandResponse>::new();

        if self.pass.is_empty() {
            return;
        }

        log::info!(
            "++++++++++++++++++++++++++++++++ redis[{}] AUTH pass({})",
            self.name,
            self.pass
        );

        let name = self.name.clone();
        self.send(
            vec!["AUTH".to_owned(), self.pass.clone()],
            move |rpl| {
                if rpl.is_string() && rpl.as_string() == "OK" {
                    log::info!("reids[{}] AUTH OK", name);
                } else {
                    log::error!("reids[{}] AUTH error!!! result: {:?}", name, rpl);
                }
            },
            Some(pinky),
        );

        prms.wait();
    }

    ///
    pub fn do_select(&mut self) {
        //
        let (prms, pinky) = PinkySwear::<CommandResponse>::new();

        if self.pass.is_empty() {
            return;
        }

        log::info!(
            "++++++++++++++++++++++++++++++++ redis[{}] SELECT dbindex({})",
            self.name,
            self.dbindex
        );

        let name = self.name.clone();
        let dbindex = self.dbindex;
        self.send(
            vec!["SELECT".to_owned(), self.dbindex.to_string()],
            move |rpl| {
                if rpl.is_string() && rpl.as_string() == "OK" {
                    log::info!("reids[{}] SELECT dbindex({}) OK", name, dbindex);
                } else {
                    log::error!(
                        "reids[{}] SELECT dbindex({}) error!!! result: {:?}",
                        name,
                        dbindex,
                        rpl
                    );
                }
            },
            Some(pinky),
        );

        prms.wait();
    }

    fn bind_conn(&mut self, conn: &Arc<TcpConn>) {
        self.conn_opt = Some(conn.clone());
    }

    fn build_commands(&mut self) {
        //
        let cmd_num = self.commands.len();
        self.buffer
            .ensure_writable_bytes(5_usize + self.commands_bytes + 5_usize * cmd_num);

        //
        for command in &self.commands {
            write!(self.buffer, "*{}\r\n", command.cmd.len()).unwrap();
            for part in &command.cmd {
                write!(self.buffer, "${}\r\n{}\r\n", part.len(), part).unwrap();
            }
        }
    }
}
