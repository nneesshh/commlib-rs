use std::collections::VecDeque;
use std::fmt::Write;
use std::sync::Arc;

use net_packet::Buffer;

use crate::{RedisReply, TcpConn};

const MAX_COMMAND_PART_NUM: usize = 64;

const CMD_BUFFER_INITIAL_SIZE: usize = 4096;
const CMD_BUFFER_RESERVED_PREPEND_SIZE: usize = 0;

///
pub type ReplyCallback = Box<dyn FnOnce(&mut RedisCommander, RedisReply) + Send + Sync>;

///
pub struct Command {
    cmd: Vec<String>,
    cb_opt: Option<ReplyCallback>,
}

///
pub struct RedisCommander {
    //
    name: String,
    pass: String,   // redis 密码
    dbindex: isize, // redis db index

    //
    conn_opt: Option<Arc<TcpConn>>,
    commands: VecDeque<Command>,
    running_cb_num: usize,

    //
    buffer: Buffer,

    //
    auth_ready: bool,
    select_ready: bool,
}

impl RedisCommander {
    ///
    pub fn new(name: &str, pass: &str, dbindex: isize) -> Self {
        Self {
            name: name.to_owned(),
            pass: pass.to_owned(),
            dbindex,

            conn_opt: None,
            commands: VecDeque::new(),
            running_cb_num: 0,

            buffer: Buffer::new(CMD_BUFFER_INITIAL_SIZE, CMD_BUFFER_RESERVED_PREPEND_SIZE),

            auth_ready: false,
            select_ready: false,
        }
    }

    /// 连接成功
    pub fn on_connect(&mut self, conn: &Arc<TcpConn>) {
        self.bind_conn(conn);
        self.do_auth();
        self.do_select();

        //
        self.do_commit();
    }

    /// 收到 reply
    pub fn on_receive_reply(&mut self, reply: RedisReply) {
        //
        if let Some(mut command) = self.commands.pop_front() {
            //
            self.running_cb_num -= 1;

            //
            let cb = command.cb_opt.take().unwrap();
            cb(self, reply);
        }
    }

    ///
    pub fn on_disconnect(&mut self) {
        //
        self.reset();
    }

    ///
    #[allow(dead_code)]
    pub fn is_auth_ready(&self) -> bool {
        self.auth_ready
    }

    ///
    pub fn set_auth_ready(&mut self, flag: bool) {
        self.auth_ready = flag;
    }

    ///
    #[allow(dead_code)]
    pub fn is_select_ready(&self) -> bool {
        self.select_ready
    }

    ///
    pub fn set_select_ready(&mut self, flag: bool) {
        self.select_ready = flag;
    }

    ///
    #[allow(dead_code)]
    pub fn is_ready(&self) -> bool {
        self.is_auth_ready() && self.is_select_ready()
    }

    ///
    pub fn do_send<F>(&mut self, cmd: Vec<String>, cb: F)
    where
        F: FnOnce(&mut RedisCommander, RedisReply) + Send + Sync + 'static,
    {
        assert!(cmd.len() > 0);
        assert!(self.commands.len() < MAX_COMMAND_PART_NUM);

        // build at once
        self.build_one_command(&cmd);

        // cache command
        let command = Command {
            cmd,
            cb_opt: Some(Box::new(cb)),
        };
        self.commands.push_back(command);
        self.running_cb_num += 1;
    }

    pub fn do_commit(&mut self) -> bool {
        //
        if let Some(conn) = self.conn_opt.as_ref() {
            let conn = conn.clone();
            if !conn.is_closed() {
                //
                let data = self.buffer.next_all();
                /*{
                    let s = unsafe { std::str::from_utf8_unchecked(data) };
                    log::info!(
                        "[do_commit]({}) send: ({}){:?} -- cmds_num={}, running_cb_num={}",
                        self.name,
                        s.len(),
                        s,
                        self.commands.len(),
                        self.running_cb_num
                    );
                }*/
                conn.send(data);

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    ///
    pub fn do_clear_commands(&mut self) {
        //
        let commander = unsafe { &mut *(self as *const Self as *mut Self) };

        //
        for command in &commander.commands {
            let cmd_tips = &command.cmd[0];

            log::info!(
                "[do_clear_commands]({}) command:{} cleared -- cmds_num={}, running_cb_num={}",
                commander.name,
                cmd_tips,
                commander.commands.len(),
                commander.running_cb_num
            );
        }

        // 清空
        commander.commands.clear();
        commander.running_cb_num = 0;
    }

    fn do_auth(&mut self) {
        //
        if self.pass.is_empty() {
            log::info!("[do_auth]({}) AUTH pass: None", self.name);

            self.set_auth_ready(true);
            return;
        }

        if let Some(conn) = &self.conn_opt {
            log::info!("[do_auth]({}) AUTH pass({})", self.name, self.pass);

            let conn = conn.clone();
            let name = self.name.clone();
            self.do_send(
                vec!["AUTH".to_owned(), self.pass.clone()],
                move |commander, rpl| {
                    if rpl.is_string() && rpl.as_string() == "OK" {
                        log::info!("[do_auth]({}) AUTH OK", name);

                        commander.set_auth_ready(true);
                    } else {
                        log::error!(
                            "[do_auth]({}) AUTH error!!! result: {:?}!!! close connection!!!",
                            name,
                            rpl
                        );

                        // low level close
                        conn.close();
                    }
                },
            );
            self.do_commit();
        } else {
            log::error!(
                "[do_auth]({}) AUTH pass({}) failed!!! conn error!!!",
                self.name,
                self.pass
            );
        }
    }

    fn do_select(&mut self) {
        //
        log::info!(
            "[do_select]({}) SELECT dbindex({})",
            self.name,
            self.dbindex
        );

        let name = self.name.clone();
        let dbindex = self.dbindex;
        self.do_send(
            vec!["SELECT".to_owned(), self.dbindex.to_string()],
            move |commander, rpl| {
                if rpl.is_string() && rpl.as_string() == "OK" {
                    log::info!("[do_select]({}) SELECT dbindex({}) OK", name, dbindex);

                    commander.set_select_ready(true);
                } else {
                    log::error!(
                        "[do_select]({}) SELECT dbindex({}) error!!! result: {:?}",
                        name,
                        dbindex,
                        rpl
                    );
                }
            },
        );
    }

    fn reset(&mut self) {
        //
        self.set_auth_ready(false);
        self.set_select_ready(false);
        self.do_clear_commands();
        self.conn_opt = None;
    }

    fn bind_conn(&mut self, conn: &Arc<TcpConn>) {
        self.conn_opt = Some(conn.clone());
    }

    fn build_one_command(&mut self, cmd: &Vec<String>) {
        // part num in a cmd vec
        let part_num = cmd.len();
        self.buffer
            .ensure_writable_bytes(5_usize + 5_usize * part_num);

        // array
        write!(self.buffer, "*{}\r\n", part_num).unwrap();

        // string item
        for part in cmd {
            write!(self.buffer, "${}\r\n{}\r\n", part.len(), part).unwrap();
        }
    }
}
