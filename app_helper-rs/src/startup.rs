//! Commlib: Startup

use parking_lot::Mutex;

/// 任务步骤回调函数
pub type StepAction = dyn FnMut() -> bool + Send + Sync + 'static;

struct StartupTask {
    desc: String, // 每个步骤加一个描述方便差错
    action: Box<StepAction>,
}

struct StartupHandle {
    name: String,
    tasks: Vec<StartupTask>,
    index: usize,
    suspending: bool,
}

impl StartupHandle {
    ///
    pub(crate) fn new(name: &str) -> StartupHandle {
        StartupHandle {
            name: name.to_owned(),
            tasks: Vec::new(),
            index: 0,
            suspending: false,
        }
    }

    ///
    pub(crate) fn exec_tasks(&mut self) {
        let task_count = self.tasks.len();
        if 0 == task_count {
            log::info!("startup[{}]: no task.", self.name);
            return;
        }

        while self.index < task_count && self.exec_step() {
            self.index += 1;

            if self.index < task_count {
                let task = &self.tasks[self.index];
                log::info!(
                    "startup[{}]: next task({}) index({}) ... ... tail_index={}",
                    self.name,
                    task.desc,
                    self.index,
                    task_count - 1
                );
            }
        }

        if self.index < task_count {
            let task = &self.tasks[self.index];
            self.suspending = true;
            log::info!(
                "startup[{}]: task({}) suspending at index({}) ... ... tail_index={}",
                self.name,
                task.desc,
                self.index,
                task_count - 1
            );
        } else {
            self.suspending = false;
            log::info!("startup[{}]: ======== over ========", self.name);
        }
    }

    fn exec_step(&mut self) -> bool {
        let task_count = self.tasks.len();
        if 0 == task_count {
            log::info!("startup[{}]: no task.", self.name);
            return true;
        }

        if self.index >= task_count {
            log::info!("startup[{}]: all task are over.", self.name);
            return true;
        }

        let task = &mut self.tasks[self.index];
        log::info!(
            "startup[{}]: exec task({}) at index({}) ... ... tail_index={}",
            self.name,
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
    /// Constructor
    pub fn new(name: &str) -> Startup {
        Startup {
            handle: Mutex::new(StartupHandle::new(name)),
        }
    }

    /// 添加启动步骤
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

    /// 执行 startup 步骤
    pub fn exec(&mut self) {
        let mut handle = self.handle.lock();
        handle.exec_tasks();
    }

    /// 挂起返回，继续执行启动步骤，注意避免死循环
    pub fn resume(&mut self) {
        let mut handle = self.handle.lock();
        if handle.suspending {
            handle.index += 1;
        }
        handle.exec_tasks();
    }
}
