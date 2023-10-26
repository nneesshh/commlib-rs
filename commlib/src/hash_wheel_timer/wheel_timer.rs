//!
//! [`WheelTimer`]: clone from [`SimulationTimer`]
//!
//! Progress in the simulation is driven by repeatedly calling the [next](WheelTimer::next) function
//! until it returns [WheelTimerSimStep::Finished](WheelTimerSimStep::Finished) indicating that the timer is empty
//! and thus the simulation has run to completion.
//!

use std::{
    fmt::Debug,
    hash::Hash,
    sync::Arc,
    time::{Duration, SystemTime},
};

use super::wheels::{cancellable::QuadWheelWithOverflow, Skip};
use super::{
    CancellableTimerEntry, OneShotClosureState, OneshotState, PeriodicClosureState, PeriodicState,
    Timer, TimerError, TimerReturn,
};

pub use super::timers::TimerEntry;

// Almost the same as `TimerEntry`, but not storing unnecessary things
impl<I, O, P> TimerEntry<I, O, P>
where
    I: Hash + Clone + Eq + Debug + Send + Sync,
    O: OneshotState<Id = I> + Debug + Send + Sync,
    P: PeriodicState<Id = I> + Debug + Send + Sync,
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

    fn execute_unique_ref(unique_ref: Arc<Self>) -> Option<(Arc<Self>, Duration)> {
        let unique = Arc::try_unwrap(unique_ref).expect("shouldn't hold on to these refs anywhere");
        unique.execute().map(|t| {
            let (new_unique, delay) = t;
            (Arc::new(new_unique), delay)
        })
    }
}

impl<I, O, P> CancellableTimerEntry for TimerEntry<I, O, P>
where
    I: Hash + Clone + Eq + Debug + Send + Sync,
    O: OneshotState<Id = I> + Debug + Send + Sync,
    P: PeriodicState<Id = I> + Debug + Send + Sync,
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
    I: Hash + Clone + Eq + Debug + Send + Sync,
    O: OneshotState<Id = I> + Debug + Send + Sync,
    P: PeriodicState<Id = I> + Debug + Send + Sync,
{
    time: u128,
    timer: QuadWheelWithOverflow<TimerEntry<I, O, P>>,
}

impl<I, O, P> WheelTimer<I, O, P>
where
    I: Hash + Clone + Eq + Debug + Send + Sync,
    O: OneshotState<Id = I> + Debug + Send + Sync,
    P: PeriodicState<Id = I> + Debug + Send + Sync,
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

    /// Update by ms
    #[inline(always)]
    pub fn update(&mut self, d: std::time::Duration) {
        let expired_vec = self.collect_expired(d);
        let to_reschedule_vec = Self::trigger_expired(expired_vec);
        self.reschedule(to_reschedule_vec);
    }

    // Update: collect expired
    #[inline(always)]
    fn collect_expired(&mut self, d: std::time::Duration) -> Vec<Arc<TimerEntry<I, O, P>>> {
        let mut expired_vec: Vec<Arc<TimerEntry<I, O, P>>> = Vec::with_capacity(256);

        let mut delta = d.as_millis() as u32;
        while delta > 0 {
            match self.timer.can_skip() {
                Skip::Empty => {
                    // Wheel is empty, or , do nothing
                    break;
                }
                Skip::None => {
                    // current tick has expiring timers, tick it (juts 1 ms)
                    delta -= 1u32;

                    let res = self.timer.tick();
                    self.time += 1u128;

                    // collect
                    for e in res {
                        expired_vec.push(e);
                    }
                }
                Skip::Millis(ms) => {
                    // skip n ms
                    let n = std::cmp::min(ms, delta);
                    delta -= n;

                    self.timer.skip(n);
                    self.time += n as u128;
                }
            }
        }

        //
        expired_vec
    }

    // Update: trigger expired
    #[inline(always)]
    fn trigger_expired(
        expired_vec: Vec<Arc<TimerEntry<I, O, P>>>,
    ) -> Vec<(Arc<TimerEntry<I, O, P>>, Duration)> {
        let mut to_reschedule_vec = Vec::with_capacity(expired_vec.len());
        for e in expired_vec {
            if let Some(pair) = TimerEntry::execute_unique_ref(e) {
                to_reschedule_vec.push(pair);
            }
        }
        to_reschedule_vec
    }

    // Update: reschedule
    #[inline(always)]
    fn reschedule(&mut self, to_reschedule_vec: Vec<(Arc<TimerEntry<I, O, P>>, Duration)>) {
        for (new_e, delay) in to_reschedule_vec {
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
    I: Hash + Clone + Eq + Debug + Send + Sync,
    O: OneshotState<Id = I> + Debug + Send + Sync,
    P: PeriodicState<Id = I> + Debug + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I> WheelTimer<I, OneShotClosureState<I>, PeriodicClosureState<I>>
where
    I: Hash + Clone + Eq + Debug + Send + Sync,
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
    I: Hash + Clone + Eq + Debug + Send + Sync,
    O: OneshotState<Id = I> + Debug + Send + Sync,
    P: PeriodicState<Id = I> + Debug + Send + Sync,
{
    type Id = I;
    type OneshotState = O;
    type PeriodicState = P;

    fn schedule_once(&mut self, timeout: Duration, state: Self::OneshotState) {
        let e = TimerEntry::OneShot { timeout, state };
        match self.timer.insert_ref_with_delay(Arc::new(e), timeout) {
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
        match self.timer.insert_ref_with_delay(Arc::new(e), delay) {
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
    use crate::hash_wheel_timer::timers::ClosureTimer;
    use crate::hash_wheel_timer::wheel_timer::*;
    use parking_lot::Mutex;
    use uuid::Uuid;
    use Arc;

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
                let mut guard = barrier.lock();
                *guard = true;
            });
        }
        let running = true;
        let mut count = 0;
        while running && count < 1000 {
            timer.update(Duration::from_millis(1));
            count += 1;
        }
        println!("Simulation run done!");
        for b in barriers {
            let guard = b.lock();
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
                    let mut guard = barrier.lock();
                    *guard = true;
                    TimerReturn::Cancel
                }
            });
        }
        let running = true;
        let mut count = 0;
        while running && count < 1000 {
            timer.update(Duration::from_millis(1));
            count += 1;
        }
        println!("Simulation run done!");
        for b in barriers {
            let guard = b.lock();
            assert!(*guard);
        }
    }
}
