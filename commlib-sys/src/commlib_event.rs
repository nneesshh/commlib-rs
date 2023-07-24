//!
//! Common Library: event
//!
//! EventDispatcher use "observer pattern"
//! Observer is a behavioral design pattern that allows one objects to notify other objects about changes in their state.

/// Trait to signal that this is an event type.
pub trait Event {
    fn add_callback<F>(&self, f: F)
    where
        F: Fn(&Self) + Send + 'static;
    fn trigger(&self);
}

/// Handler of event
pub struct EventHandler<E>
where
    E: Event,
{
    pub func: Box<dyn Fn(&E) + Send + 'static>,
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
        F: Fn(&E) + Send + 'static,
    {
        Self {
            func: Box::new(f),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Call
    pub fn handle(&self, e: &E) {
        let repeat = "*".repeat(20);
        println!("{} Begin {}", repeat, repeat);
        (self.func)(e);
        println!("{} End   {}", repeat, repeat);
    }
}

/// Listener of event
pub struct EventListener<E>
where
    E: Event,
{
    handlers: Vec<EventHandler<E>>,
}

/// Impl EventListener
impl<E> EventListener<E>
where
    E: Event,
{
    pub fn new() -> EventListener<E> {
        EventListener::<E> {
            handlers: Vec::new(),
        }
    }

    pub fn listen_event(&mut self, h: EventHandler<E>) {
        self.handlers.push(h);
    }

    pub fn call(&self, e: &E) {
        for h in &self.handlers {
            h.handle(e);
        }
    }
}

/// Impl Event trait for struct
#[macro_export]
macro_rules! impl_event_for {
    ($t:ident) => {
        paste::paste! {
            impl Event for $t {
                fn add_callback<F>(&self, f: F)
                where
                    F: Fn(&Self) + Send + 'static,
                {
                    let h = EventHandler::<Self>::new(f);
                    [<G_EVENT_LISTENER_ $t:upper>].with(|g| g.borrow_mut().listen_event(h));
                }
                fn trigger(&self) {
                    [<G_EVENT_LISTENER_ $t:upper>].with(|g| g.borrow().call(self));
                }
            }
            thread_local! {
                static [<G_EVENT_LISTENER_ $t:upper>]: std::cell::RefCell<EventListener<$t>> = std::cell::RefCell::new(EventListener::<$t>::new());
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
