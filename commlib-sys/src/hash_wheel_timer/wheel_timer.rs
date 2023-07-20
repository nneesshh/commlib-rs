//!
//! [`WheelTimer`]: clone from [`SimulationTimer`]
//!
//! Progress in the simulation is driven by repeatedly calling the [next](WheelTimer::next) function
//! until it returns [WheelTimerSimStep::Finished](WheelTimerSimStep::Finished) indicating that the timer is empty
//! and thus the simulation has run to completion.
//!
//! # Example
//! ```
//! # use std::sync::{Arc, Mutex};
//! # use uuid::Uuid;
//! # use std::time::Duration;
//! use commlib::hash_wheel_timer::*;
//! use commlib::hash_wheel_timer::wheel_timer::*;
//!
//! let mut timer = WheelTimer::for_uuid_closures();
//!
//! let barrier: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
//! let barrier2 = barrier.clone();
//! let id = Uuid::new_v4();
//! let delay = Duration::from_millis(150);
//! timer.schedule_action_once(id, delay, move |timer_id|{
//!     println!("Timer function was triggered! Id={:?}", timer_id);
//!     let mut guard = barrier2.lock().unwrap();
//!     *guard = true;
//! });
//! println!("Starting wheel timer sim run...");
//! let mut running = true;
//! while running {
//!     match timer.next() {
//!         WheelTimerSimStep::Ok => println!("Next!"),
//!         WheelTimerSimStep::Finished => running = false,
//!     }
//! }
//! println!("Wheel timer sim run done!");
//! let guard = barrier.lock().unwrap();
//! assert_eq!(*guard, true);
//! ```
use super::wheels::{cancellable::*, *};
use super::*;

use std::{
    fmt::Debug,
    hash::Hash,
    rc::Rc,
    time::{Duration, SystemTime},
};

// Almost the same as `TimerEntry`, but not storing unnecessary things
impl<I, O, P> timers::TimerEntry<I, O, P>
where
    I: Hash + Clone + Eq + Debug,
    O: OneshotState<Id = I> + Debug,
    P: PeriodicState<Id = I> + Debug,
{
    fn execute(self) -> Option<(Self, Duration)> {
        match self {
            TimerEntry::OneShot { timeout: _, state } => {
                state.trigger();
                None
            }
            TimerEntry::Periodic {
                delay,
                period,
                state,
            } => match state.trigger() {
                TimerReturn::Reschedule(new_state) => {
                    let new_entry = TimerEntry::Periodic {
                        delay,
                        period,
                        state: new_state,
                    };
                    Some((new_entry, period))
                }
                TimerReturn::Cancel => None,
            },
        }
    }

    fn execute_unique_ref(unique_ref: Rc<Self>) -> Option<(Rc<Self>, Duration)> {
        let unique = Rc::try_unwrap(unique_ref).expect("shouldn't hold on to these refs anywhere");
        unique.execute().map(|t| {
            let (new_unique, delay) = t;
            (Rc::new(new_unique), delay)
        })
    }
}

impl<I, O, P> CancellableTimerEntry for TimerEntry<I, O, P>
where
    I: Hash + Clone + Eq + Debug,
    O: OneshotState<Id = I> + Debug,
    P: PeriodicState<Id = I> + Debug,
{
    type Id = I;

    fn id(&self) -> &Self::Id {
        match self {
            TimerEntry::OneShot { state, .. } => state.id(),
            TimerEntry::Periodic { state, .. } => state.id(),
        }
    }
}

/// A timer implementation that used timing-wheel
pub struct WheelTimer<I, O, P>
where
    I: Hash + Clone + Eq + Debug,
    O: OneshotState<Id = I> + Debug,
    P: PeriodicState<Id = I> + Debug,
{
    time: u128,
    timer: QuadWheelWithOverflow<TimerEntry<I, O, P>>,
}

impl<I, O, P> WheelTimer<I, O, P>
where
    I: Hash + Clone + Eq + Debug,
    O: OneshotState<Id = I> + Debug,
    P: PeriodicState<Id = I> + Debug,
{
    /// Create a new simulation timer starting at `0`
    pub fn new() -> Self {
        WheelTimer {
            time: 0u128,
            timer: QuadWheelWithOverflow::new(),
        }
    }

    /// Create a new simulation timer starting at a system clock value
    pub fn at(now: SystemTime) -> Self {
        let t = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH!");
        let tms = t.as_millis();
        WheelTimer {
            time: tms,
            timer: QuadWheelWithOverflow::new(),
        }
    }

    /// Return the timers current virtual time value (in ms)
    pub fn current_time(&self) -> u128 {
        self.time
    }

    /// Advance the virtual time
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> WheelTimerSimStep {
        loop {
            match self.timer.can_skip() {
                Skip::Empty => return WheelTimerSimStep::Finished,
                Skip::None => {
                    let res = self.timer.tick();
                    self.time += 1u128;
                    if !res.is_empty() {
                        for e in res {
                            self.trigger_entry(e);
                        }
                        return WheelTimerSimStep::Ok;
                    }
                }
                Skip::Millis(ms) => {
                    self.timer.skip(ms);
                    self.time += ms as u128;
                    let res = self.timer.tick();
                    self.time += 1u128;
                    if !res.is_empty() {
                        for e in res {
                            self.trigger_entry(e);
                        }
                        return WheelTimerSimStep::Ok;
                    }
                }
            }
        }
    }

    /// Update by ms
    #[allow(clippy::should_implement_trait)]
    pub fn update(&mut self, d: std::time::Duration) {
        let mut delta = d.as_millis() as u32;
        while delta > 0 {
            match self.timer.can_skip() {
                Skip::Empty => {
                    // Wheel is empty, do nothing
                    break
                },
                Skip::None => {
                    // tick 1 ms
                    delta -= 1u32;

                    let res = self.timer.tick();
                    self.time += 1u128;
                    if !res.is_empty() {
                        for e in res {
                            self.trigger_entry(e);
                        }
                    }
                }
                Skip::Millis(ms) => {
                    // skip n ms
                    let n = if ms < delta {
                        delta - ms
                    } else {
                        delta
                    };
                    delta -= n;

                    self.timer.skip(n);
                    self.time += n as u128;
                }
            }
        }
    }

    fn trigger_entry(&mut self, e: Rc<TimerEntry<I, O, P>>) {
        if let Some((new_e, delay)) = TimerEntry::execute_unique_ref(e) {
            match self.timer.insert_ref_with_delay(new_e, delay) {
                Ok(_) => (), // ok
                Err(TimerError::Expired(e)) => panic!(
                    "Trying to insert periodic timer entry with 0ms period! {:?}",
                    e
                ),
                Err(f) => panic!("Could not insert timer entry! {:?}", f),
            }
        } // otherwise, timer is not rescheduled
    }
}

impl<I, O, P> Default for WheelTimer<I, O, P>
where
    I: Hash + Clone + Eq + Debug,
    O: OneshotState<Id = I> + Debug,
    P: PeriodicState<Id = I> + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I> WheelTimer<I, OneShotClosureState<I>, PeriodicClosureState<I>>
where
    I: Hash + Clone + Eq + Debug,
{
    /// Shorthand for creating a simulation timer using closure state
    pub fn for_closures() -> Self {
        Self::new()
    }
}

#[cfg(feature = "uuid-extras")]
impl WheelTimer<uuid::Uuid, OneShotClosureState<uuid::Uuid>, PeriodicClosureState<uuid::Uuid>> {
    /// Shorthand for creating a simulation timer using Uuid identifiers and closure state
    pub fn for_uuid_closures() -> Self {
        Self::new()
    }
}

/// Result of advancing virtual time
pub enum WheelTimerSimStep {
    /// No timer entries remain
    ///
    /// The simulation can be considered complete.
    Finished,
    /// Step was executed, but more timer entries remain
    ///
    /// Continue calling [next](WheelTimer::next) to advance virtual time.
    Ok,
}

impl<I, O, P> Timer for WheelTimer<I, O, P>
where
    I: Hash + Clone + Eq + Debug,
    O: OneshotState<Id = I> + Debug,
    P: PeriodicState<Id = I> + Debug,
{
    type Id = I;
    type OneshotState = O;
    type PeriodicState = P;

    fn schedule_once(&mut self, timeout: Duration, state: Self::OneshotState) {
        let e = TimerEntry::OneShot { timeout, state };
        match self.timer.insert_ref_with_delay(Rc::new(e), timeout) {
            Ok(_) => (), // ok
            Err(TimerError::Expired(e)) => {
                if TimerEntry::execute_unique_ref(e).is_none() {
                    // do nothing
                } else {
                    // clearly a OneShot
                    unreachable!("OneShot produced reschedule!")
                }
            }
            Err(f) => panic!("Could not insert timer entry! {:?}", f),
        }
    }

    fn schedule_periodic(&mut self, delay: Duration, period: Duration, state: Self::PeriodicState) {
        let e = TimerEntry::Periodic {
            delay,
            period,
            state,
        };
        match self.timer.insert_ref_with_delay(Rc::new(e), delay) {
            Ok(_) => (), // ok
            Err(TimerError::Expired(e)) => {
                if let Some((new_e, delay)) = TimerEntry::execute_unique_ref(e) {
                    match self.timer.insert_ref_with_delay(new_e, delay) {
                        Ok(_) => (), // ok
                        Err(TimerError::Expired(e)) => panic!(
                            "Trying to insert periodic timer entry with 0ms period! {:?}",
                            e
                        ),
                        Err(f) => panic!("Could not insert timer entry! {:?}", f),
                    }
                }
            } // otherwise, timer decided not to reschedule itself
            Err(f) => panic!("Could not insert timer entry! {:?}", f),
        }
    }

    fn cancel(&mut self, id: &Self::Id) {
        match self.timer.cancel(id) {
            Ok(_) => (),                                                             // great
            Err(f) => eprintln!("Could not cancel timer with id={:?}. {:?}", id, f), // not so great, but meh
        }
    }
}

#[cfg(feature = "uuid-extras")]
#[cfg(test)]
mod tests {
    use crate::hash_wheel_timer::test_helpers::*;
    use crate::hash_wheel_timer::wheel_timer::*;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[test]
    fn simple_simulation() {
        let num = 10usize;
        let mut barriers: Vec<Arc<Mutex<bool>>> = Vec::with_capacity(num);
        let mut timer = WheelTimer::for_uuid_closures();
        for i in 0..num {
            let barrier = Arc::new(Mutex::new(false));
            barriers.push(barrier.clone());
            let id = Uuid::new_v4();
            let timeout = fib_time(i);
            timer.schedule_action_once(id, timeout, move |_| {
                println!("Running action {}", i);
                let mut guard = barrier.lock().unwrap();
                *guard = true;
            });
        }
        let mut running = true;
        while running {
            match timer.next() {
                WheelTimerSimStep::Ok => println!("Next!"),
                WheelTimerSimStep::Finished => running = false,
            }
        }
        println!("Simulation run done!");
        for b in barriers {
            let guard = b.lock().unwrap();
            assert!(*guard);
        }
    }

    #[test]
    fn rescheduling_simulation() {
        let num = 10usize;
        let mut barriers: Vec<Arc<Mutex<bool>>> = Vec::with_capacity(num);
        let mut timer = WheelTimer::for_uuid_closures();
        for i in 1..num {
            let barrier = Arc::new(Mutex::new(false));
            barriers.push(barrier.clone());
            let id = Uuid::new_v4();
            let timeout = fib_time(i);
            let mut counter: usize = 5;
            timer.schedule_action_periodic(id, timeout, timeout, move |_| {
                println!("Running action {}", i);
                if counter > 0 {
                    counter -= 1;
                    TimerReturn::Reschedule(())
                } else {
                    let mut guard = barrier.lock().unwrap();
                    *guard = true;
                    TimerReturn::Cancel
                }
            });
        }
        let mut running = true;
        while running {
            match timer.next() {
                WheelTimerSimStep::Ok => println!("Next!"),
                WheelTimerSimStep::Finished => running = false,
            }
        }
        println!("Simulation run done!");
        for b in barriers {
            let guard = b.lock().unwrap();
            assert!(*guard);
        }
    }
}
