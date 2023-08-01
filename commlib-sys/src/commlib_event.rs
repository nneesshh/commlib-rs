//!
//! Common Library: event
//!
//! EventDispatcher use "observer pattern"
//! Observer is a behavioral design pattern that allows one objects to notify other objects about changes in their state.

use crate::StopWatch;

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
    pub func: Box<dyn FnMut(&E) + 'static>,
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
    S: crate::ServiceRs,
    E: Event,
{
    handlers: Vec<EventHandler<E>>,
    _phantom: std::marker::PhantomData<S>,
}

/// Impl EventListener
impl<S, E> EventListener<S, E>
where
    S: crate::ServiceRs,
    E: Event,
{
    pub fn new() -> EventListener<S, E> {
        EventListener::<S, E> {
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
            let typeid = e.clone();
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

#[cfg(test)]
mod event_tests {
    use crate::commlib_event::*;

    struct MyEvent {
        code: u32,
    }
    impl_event_for!(MyEvent);

    #[test]
    fn trigger_simple_event() {
        let e = MyEvent { code: 123 };
        e.add_callback(move |_| println!("{}", e.code));
        e.trigger();

        //
        let h = EventHandler::<MyEvent>::new(move |_| println!("{}", e.code));
        G_EVENT_LISTENER_MYEVENT.with(|g| g.borrow_mut().listen_event(h));
    }
}
