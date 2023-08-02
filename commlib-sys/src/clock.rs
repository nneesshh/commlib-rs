//!
//! Clock
//!

use crate::hash_wheel_timer::{self, ClosureTimer, TimerReturn::Reschedule};
use std::ops::AddAssign;

pub type WheelTimer = hash_wheel_timer::wheel_timer::WheelTimer<
    uuid::Uuid,
    hash_wheel_timer::OneShotClosureState<uuid::Uuid>,
    hash_wheel_timer::PeriodicClosureState<uuid::Uuid>,
>;

//
pub struct TimerAction<F>
where
    F: FnMut() + Send,
{
    pub func: F,
}

/// Clock utils
pub struct Clock {
    wheel_timer: WheelTimer,
    last_time: std::time::SystemTime,
}

impl Clock {
    /// New clock
    pub fn new() -> Self {
        Self {
            wheel_timer: WheelTimer::new(),
            last_time: std::time::SystemTime::now(),
        }
    }

    /// 循环定时器，立即执行一次
    pub fn set_timer<F>(&mut self, interval: u64, mut action: TimerAction<F>)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        let id = uuid::Uuid::new_v4();
        let delay = std::time::Duration::from_millis(0);
        let period = std::time::Duration::from_millis(interval);
        self.wheel_timer
            .schedule_action_periodic(id, delay, period, move |_timer_id| {
                (action.func)();
                Reschedule(())
            })
    }

    /// 循环定时器，延时一段时间之后开始执行
    pub fn set_timer_delay<F>(&mut self, delay: u64, interval: u64, mut action: TimerAction<F>)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        let id = uuid::Uuid::new_v4();
        let delay = std::time::Duration::from_millis(delay);
        let period = std::time::Duration::from_millis(interval);
        self.wheel_timer
            .schedule_action_periodic(id, delay, period, move |_timer_id| {
                (action.func)();
                Reschedule(())
            })
    }

    /// One-shot 一次性超时
    pub fn set_timeout<F>(&mut self, delay: u64, mut action: TimerAction<F>)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        let id = uuid::Uuid::new_v4();
        let delay = std::time::Duration::from_millis(delay);
        self.wheel_timer
            .schedule_action_once(id, delay, move |_timer_id| {
                (action.func)();
            })
    }

    /// 更新计时器 tick
    pub fn update(&mut self) {
        match self.last_time.elapsed() {
            Ok(d) => {
                self.wheel_timer.update(d);
                self.last_time.add_assign(d);
            }
            Err(err) => {
                log::error!("clock error: {:?}!!!", err);
            }
        }
    }
}
