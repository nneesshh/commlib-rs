//! Commlib: Startup

use parking_lot::Mutex;

/// 任务步骤回调函数
pub type StepAction = dyn FnMut() -> bool + Send + Sync + 'static;

struct StartupTask {
    desc: String, // 每个步骤加一个描述方便差错
    action: Box<StepAction>,
}

struct StartupHandle {
    srv_id: i32,
    tasks: Vec<StartupTask>,
    index: usize,
    suspending: bool,
}

impl StartupHandle {
    ///
    pub(crate) fn new(srv_id: i32) -> StartupHandle {
        StartupHandle {
            srv_id,
            tasks: Vec::new(),
            index: 0,
            suspending: false,
        }
    }

    ///
    pub(crate) fn clear(&mut self) {
        self.tasks.clear();
        self.index = 0;
        self.suspending = false;
    }

    ///
    pub(crate) fn exec_tasks(&mut self) {
        let task_count = self.tasks.len();
        if 0 == task_count {
            log::info!("startup[{}]: no task.", self.srv_id);
            return;
        }

        while self.index < task_count && self.exec_step() {
            self.index += 1;

            if self.index < task_count {
                let task = &self.tasks[self.index];
                log::debug!(
                    "startup[{}]: next task({}) index({}) ... ... tail_index={}",
                    self.srv_id,
                    task.desc,
                    self.index,
                    task_count - 1
                );
            }
        }

        if self.index < task_count {
            let task = &self.tasks[self.index];
            self.suspending = true;
            log::debug!(
                "startup[{}]: task({}) suspending at index({}) ... ... tail_index={}",
                self.srv_id,
                task.desc,
                self.index,
                task_count - 1
            );
        } else {
            self.suspending = false;
            log::debug!("startup[{}]: ======== over ========", self.srv_id);
        }
    }

    fn exec_step(&mut self) -> bool {
        let task_count = self.tasks.len();
        if 0 == task_count {
            log::info!("startup[{}]: no task.", self.srv_id);
            return true;
        }

        if self.index >= task_count {
            log::info!("startup[{}]: all task are over.", self.srv_id);
            return true;
        }

        let task = &mut self.tasks[self.index];
        log::debug!(
            "startup[{}]: exec task({}) at index({}) ... ... tail_index={}",
            self.srv_id,
            task.desc,
            self.index,
            task_count - 1
        );

        // exec
        (task.action)()
    }
}

/// 启动步骤
pub struct Startup {
    handle: Mutex<StartupHandle>,
}

impl Startup {
    ///
    pub fn new(srv_id: i32) -> Startup {
        Startup {
            handle: Mutex::new(StartupHandle::new(srv_id)),
        }
    }

    ///
    pub fn add_step<F>(&mut self, desc: &str, action: F)
    where
        F: FnMut() -> bool + Send + Sync + 'static,
    {
        let task = StartupTask {
            desc: desc.to_owned(),
            action: Box::new(action),
        };
        let mut handle = self.handle.lock();
        handle.tasks.push(task)
    }

    ///
    pub fn run(&mut self) {
        let mut handle = self.handle.lock();
        handle.exec_tasks();
    }

    ///
    pub fn resume(&mut self) {
        let mut handle = self.handle.lock();
        if handle.suspending {
            handle.index += 1;
        }
        handle.exec_tasks();
    }

    ///
    pub fn clear(&mut self) {
        let mut handle = self.handle.lock();
        handle.clear();
    }
}
