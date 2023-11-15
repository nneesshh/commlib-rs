//!
//! Clock
//!

use std::cell::UnsafeCell;
use std::ops::{AddAssign, SubAssign};
use std::time::SystemTime;

use crate::hash_wheel_timer::{self, ClosureTimer, TimerReturn::Reschedule};
use crate::ServiceRs;

pub type WheelTimer = hash_wheel_timer::wheel_timer::WheelTimer<
    uuid::Uuid,
    hash_wheel_timer::OneShotClosureState<uuid::Uuid>,
    hash_wheel_timer::PeriodicClosureState<uuid::Uuid>,
>;

pub type WheelTimerEntry = hash_wheel_timer::wheel_timer::TimerEntry<
    uuid::Uuid,
    hash_wheel_timer::OneShotClosureState<uuid::Uuid>,
    hash_wheel_timer::PeriodicClosureState<uuid::Uuid>,
>;

thread_local! {
    /// tls 时钟
    pub static G_CLOCK: UnsafeCell<Clock> = {
        UnsafeCell::new(Clock::new())
    };
}

/// Clock utils
pub struct Clock {
    // 时间轮
    wheel_timer: WheelTimer,

    // 用于计算 elapsed
    last_time: SystemTime,

    // now stamp
    now_stamp: u64,
}

impl Clock {
    /// New clock
    pub fn new() -> Self {
        let now = SystemTime::now();
        let now_stamp = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            wheel_timer: WheelTimer::new(),
            last_time: now,
            now_stamp,
        }
    }

    /// 循环定时器，立即执行一次
    pub fn set_timer<T, F>(srv: &T, interval: u64, mut f: F)
    where
        T: ServiceRs + 'static,
        F: FnMut() + Send + Sync + 'static,
    {
        // post 到指定 srv 工作线程中执行
        srv.run_in_service(Box::new(move || {
            with_tls_mut!(G_CLOCK, clock, {
                let wheel_timer = &mut clock.wheel_timer;

                let id = uuid::Uuid::new_v4();
                let delay = std::time::Duration::from_millis(0);
                let period = std::time::Duration::from_millis(interval);

                wheel_timer.schedule_action_periodic(id.clone(), delay, period, move |_timer_id| {
                    f();
                    Reschedule(())
                });
            });
        }));
    }

    /// 循环定时器，延时一段时间之后开始执行
    pub fn set_timer_delay<T, F>(srv: &T, delay: u64, interval: u64, mut f: F)
    where
        T: ServiceRs + 'static,
        F: FnMut() + Send + Sync + 'static,
    {
        // post 到指定 srv 工作线程中执行
        srv.run_in_service(Box::new(move || {
            with_tls_mut!(G_CLOCK, clock, {
                let wheel_timer = &mut clock.wheel_timer;

                let id = uuid::Uuid::new_v4();
                let delay = std::time::Duration::from_millis(delay);
                let period = std::time::Duration::from_millis(interval);

                wheel_timer.schedule_action_periodic(id, delay, period, move |_timer_id| {
                    f();
                    Reschedule(())
                });
            });
        }));
    }

    /// One-shot 一次性超时
    pub fn set_timeout<T, F>(srv: &T, delay: u64, f: F)
    where
        T: ServiceRs + 'static,
        F: FnOnce() + Send + Sync + 'static,
    {
        // post 到指定 srv 工作线程中执行
        srv.run_in_service(Box::new(move || {
            with_tls_mut!(G_CLOCK, clock, {
                let wheel_timer = &mut clock.wheel_timer;

                let id = uuid::Uuid::new_v4();
                let delay = std::time::Duration::from_millis(delay);

                wheel_timer.schedule_action_once(id, delay, move |_timer_id| {
                    f();
                });
            });
        }));
    }

    /// 更新计时器 tick
    #[inline(always)]
    pub fn update() {
        with_tls_mut!(G_CLOCK, clock, {
            //
            match clock.last_time.elapsed() {
                Ok(d) => {
                    // wheel timer update
                    clock.wheel_timer.update(d);

                    // advance last time
                    clock.last_time.add_assign(d);

                    // advance now stamp
                    clock.now_stamp += d.as_millis() as u64;
                }
                Err(err) => {
                    //
                    let d = err.duration();
                    clock.last_time.sub_assign(d);

                    //
                    log::error!(
                        "clock update error: {:?}!!! turn back last time to {:?}",
                        err,
                        clock.last_time
                    );
                }
            }
        });
    }

    ///
    pub fn now_stamp() -> u64 {
        with_tls_mut!(G_CLOCK, clock, { clock.now_stamp })
    }
}
