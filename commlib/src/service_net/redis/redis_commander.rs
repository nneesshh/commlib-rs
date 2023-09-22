use std::collections::VecDeque;
use std::fmt::Write;
use std::sync::Arc;
use atomic::{Atomic, Ordering};

use crate::pinky_swear::{Pinky, PinkySwear};
use crate::{Buffer, RedisReply, ServiceRs, TcpConn, ServiceNetRs};

const MAX_COMMAND_PART_NUM: usize = 64;

const CMD_BUFFER_INITIAL_SIZE: usize = 4096;
const CMD_BUFFER_RESERVED_PREPEND_SIZE: usize = 0;

///
pub type ReplyCallback = Box<dyn Fn(RedisReply) + Send + Sync>;

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
    srv_net:Arc<ServiceNetRs>,
    srv: Arc<dyn ServiceRs>,

    //
    conn_opt: Option<Arc<TcpConn>>,
    commands: VecDeque<Command>,
    running_cb_num: usize,

    //
    buffer: Buffer,

    //
    ready: Atomic<bool>,
}

impl RedisCommander {
    ///
    pub fn new<T>(srv: &Arc<T>, name: &str, pass: &str, dbindex: isize, srv_net:&Arc<ServiceNetRs>) -> Self
    where
        T: ServiceRs + 'static,
    {
        Self {
            name: name.to_owned(),
            pass: pass.to_owned(),
            dbindex,

            srv_net: srv_net.clone(),
            srv: srv.clone(),

            conn_opt: None,
            commands: VecDeque::new(),
            running_cb_num: 0,

            buffer: Buffer::new(CMD_BUFFER_INITIAL_SIZE, CMD_BUFFER_RESERVED_PREPEND_SIZE),

            ready: Atomic::new(false),
        }
    }

    /// 连接成功
    pub fn on_connect(&self, conn: &Arc<TcpConn>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        let commander = unsafe { &mut *(self as *const Self as *mut Self) };

        //
        commander.bind_conn(conn);
        commander.do_auth();
        commander.do_select();

        commander.set_ready(true);
    }

    /// 收到 reply
    pub fn on_receive_reply(&self, reply: RedisReply) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        let commander = unsafe { &mut *(self as *const Self as *mut Self) };
        if let Some(mut command) = commander.commands.pop_front() {
            let cb = command.cb_opt.take().unwrap();
            cb(reply);

            //
            commander.running_cb_num -= 1;
        }
    }

    ///
    pub fn on_disconnect(&self) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        let commander = unsafe { &mut *(self as *const Self as *mut Self) };

        //
        commander.set_ready(false);
        commander.conn_opt = None;
    }

    ///
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    ///
    pub fn set_ready(&self, flag: bool) {
        self.ready.store(flag, Ordering::Relaxed)
    }

    ///
    pub fn send<F>(self:&Arc<Self>, cmd: Vec<String>, cb: F)
    where
        F: Fn(RedisReply) + Send + Sync + 'static,
    {
        let cli = self.clone();
        self.srv_net.run_in_service(Box::new(move ||{
            cli.do_send(cmd, cb);
        }));        
    }

    /// 异步提交： 如果提交失败，则在连接成功后再提交一次
    pub fn commit(self:&Arc<Self>, ) {
        let cli = self.clone();
        self.srv_net.run_in_service(Box::new(move ||{
            cli.do_commit();
        }));        
    }

    ///
    pub fn send_and_commit_blocking<F>(self:&Arc<Self>, cmd: Vec<String>) -> PinkySwear<RedisReply>
    where
        F: Fn(RedisReply) + Send + Sync + 'static,
    {
        // MUST not in srv_net thread，防止 blocking 导致死锁
        assert!(!self.srv_net.is_in_service_thread());

        let (prms, pinky) = PinkySwear::<RedisReply>::new();

        let cli = self.clone();
        self.srv_net.run_in_service(Box::new(move ||{
            cli.do_send(cmd, move|reply| {
                pinky.swear(reply);
            });
        }));   

        prms     
    }

    /// 用户手工调用，清除 commands 缓存
    pub fn clear_commands(self:&Arc<Self>, ) {
        let cli = self.clone();
        self.srv.run_in_service(Box::new(move ||{
            cli.do_clear_commands();
        }));   
    }

    fn do_auth(&mut self) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        if self.pass.is_empty() {
            log::info!("redis[{}] AUTH pass: None", self.name,);
            return;
        }

        if let Some(conn) = &self.conn_opt {
            log::info!(
                "++++++++++++++++++++++++++++++++ redis[{}] AUTH pass({})",
                self.name,
                self.pass
            );

            let conn = conn.clone();
            let name = self.name.clone();
            self.do_send(vec!["AUTH".to_owned(), self.pass.clone()], move |rpl| {
                if rpl.is_string() && rpl.as_string() == "OK" {
                    log::info!("reids[{}] AUTH OK", name);
                } else {
                    log::error!("reids[{}] AUTH error!!! result: {:?}", name, rpl);

                    // disconnect
                    conn.close();
                }
            });
            self.do_commit();
        } else {
            log::error!(
                "redis[{}] AUTH pass({}) failed!!! conn error!!!",
                self.name,
                self.pass
            );
        }
    }

    fn do_select(&mut self) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

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
        self.do_send(
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
        );
        self.do_commit();
    }

    fn bind_conn(&mut self, conn: &Arc<TcpConn>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        self.conn_opt = Some(conn.clone());
    }

    fn build_one_command(&mut self, cmd: &Vec<String>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

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

    fn do_send<F>(&self, cmd: Vec<String>, cb: F)
    where
        F: Fn(RedisReply) + Send + Sync + 'static,
    {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        let commander = unsafe { &mut *(self as *const Self as *mut Self) };

        assert!(cmd.len() > 0);
        assert!(commander.commands.len() < MAX_COMMAND_PART_NUM);

        commander.build_one_command(&cmd);
        commander.commands.push_back(Command {
            cmd,
            cb_opt: Some(Box::new(cb)),
        });
        commander.running_cb_num += 1;
    }

    pub fn do_commit(&self) -> bool {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        let commander = unsafe { &mut *(self as *const Self as *mut Self) };

        if let Some(conn) = commander.conn_opt.as_ref() {
            let conn = conn.clone();
            if !conn.is_closed() {
                //
                let data = commander.buffer.next_all();
                {
                    let s = unsafe { std::str::from_utf8_unchecked(data) };
                    log::info!("redis[{}] send: ({}){:?}", self.name, s.len(), s);
                }
                conn.send(data);

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn do_clear_commands(&self) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        let commander = unsafe { &mut *(self as *const Self as *mut Self) };

        //
        for command in &commander.commands {
            let cmd_tips = &command.cmd[0];

            log::info!(
                "redis[{}] command:{} cleared, total={} running={}",
                commander.name,
                cmd_tips,
                commander.commands.len(),
                commander.running_cb_num
            );
        }

        // 清空
        commander.commands.clear();
    }

}
