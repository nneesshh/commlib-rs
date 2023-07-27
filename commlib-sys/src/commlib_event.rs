//!
//! Common Library: event
//!
//! EventDispatcher use "observer pattern"
//! Observer is a behavioral design pattern that allows one objects to notify other objects about changes in their state.

/// Trait to signal that this is an event type.
pub trait Event {
    type Host;
    /// Add callback for event
    fn add_callback<'a, F>(host: &'a mut Self::Host, f: F)
    where
        F: FnMut(&Self) + 'static;

    /// Trigger event callback
    fn trigger<'a>(&mut self, host: &'a Self::Host);
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

    pub fn call(&mut self, e: &E) {
        for h in &mut self.handlers {
            h.handle(e);
        }
    }
}

/// Impl Event trait for struct
// #[macro_export]
// macro_rules! impl_event_for {
//     ($t:ident) => {
//         paste::paste! {
//             impl Event for $t {
//                 fn add_callback<F>(f: F)
//                 where
//                     F: FnMut(&Self) + 'static,
//                 {
//                     let h = EventHandler::<Self>::new(f);
//                     [<G_EVENT_LISTENER_ $t:upper>].with(|g| g.borrow_mut().listen_event(h));
//                 }
//                 fn trigger(&mut self) {
//                     [<G_EVENT_LISTENER_ $t:upper>].with(|g| g.borrow_mut().call(self));
//                 }
//             }
//             thread_local! {
//                 static [<G_EVENT_LISTENER_ $t:upper>]: std::cell::RefCell<EventListener<$t>> = std::cell::RefCell::new(EventListener::<$t>::new());
//             }
//         }
//     };
// }

pub trait EventHost {
    /// 注册事件 callback
    fn listen_event<E>(&mut self, h: crate::EventHandler<E>) where E: crate::Event;

    /// 执行事件 callback
    fn call<E>(&self, e: &E) where E: crate::Event;
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
