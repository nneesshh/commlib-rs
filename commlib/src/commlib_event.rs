//!
//! Common Library: event
//!
//! EventDispatcher use "observer pattern"
//! Observer is a behavioral design pattern that allows one objects to notify other objects about changes in their state.

use crate::{ServiceRs, StopWatch};

/// Trait to signal that this is an event type.
pub trait Event {
    /// Id string
    fn id(&self) -> &str;

    /// Add callback for event
    fn add_callback<'a, F>(f: F)
    where
        F: FnMut(&Self) + 'static;

    /// Trigger event callback
    fn trigger<'a>(&mut self);
}

/// Handler of event
pub struct EventHandler<E>
where
    E: Event,
{
    pub func: Box<dyn FnMut(&E)>,
    _phantom: std::marker::PhantomData<E>,
}

/// Impl EventHandler
impl<E> EventHandler<E>
where
    E: Event,
{
    // Construct
    pub fn new<F>(f: F) -> Self
    where
        F: FnMut(&E) + 'static,
    {
        Self {
            func: Box::new(f),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Call
    pub fn handle(&mut self, e: &E) {
        let repeat = "*".repeat(20);
        println!("{} Begin {}", repeat, repeat);
        (self.func)(e);
        println!("{} End   {}", repeat, repeat);
    }
}

/// Listener of event
pub struct EventListener<S, E>
where
    S: ServiceRs,
    E: Event,
{
    handlers: Vec<EventHandler<E>>,
    _phantom: std::marker::PhantomData<S>,
}

/// Impl EventListener
impl<S, E> EventListener<S, E>
where
    S: ServiceRs,
    E: Event,
{
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn listen_event(&mut self, h: EventHandler<E>) {
        self.handlers.push(h);
    }

    pub fn call(&mut self, e: &E) {
        let sw = StopWatch::new();
        for h in &mut self.handlers {
            h.handle(e);
        }

        let cost = sw.elapsed();
        if cost > 10_u128 {
            log::error!(
                "call on event ID={} timeout cost: {}ms, hotspot **@{}:{}",
                e.id(),
                cost,
                std::file!(), //TODO: rela filename
                std::line!()  //TODO: real linenumber
            )
        }
    }
}

/// Impl Event trait for struct
#[macro_export]
macro_rules! impl_event_for {
    ($s:ident, $t:ident) => {
        paste::paste! {
            impl Event for $t {
                /// Id string
                fn id(&self) -> &str {
                    stringify!([<$s _ $t>])
                }
                fn add_callback<F>(f: F)
                where
                    F: FnMut(&Self) + 'static,
                {
                    let h = EventHandler::<Self>::new(f);
                    [<G_EVENT_LISTENER_ $s:upper _ $t:upper>].with(|g| g.borrow_mut().listen_event(h));
                }
                fn trigger(&mut self) {
                    [<G_EVENT_LISTENER_ $s:upper _ $t:upper>].with(|g| g.borrow_mut().call(self));
                }
            }
            thread_local! {
                static [<G_EVENT_LISTENER_ $s:upper _ $t:upper>]: std::cell::RefCell<EventListener<$s, $t>> = std::cell::RefCell::new(EventListener::<$s, $t>::new());
            }
        }
    };
}
