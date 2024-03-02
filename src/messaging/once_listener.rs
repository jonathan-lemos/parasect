use crate::messaging::listener::Listener;
use crate::messaging::listener::ListenerBehavior::StopProcessing;
use crate::threading::single_use_cell::SingleUseCell;
use crossbeam_channel::Receiver;
use std::thread::Scope;

/// Waits for a message, then runs a function in response to it.
///
/// All subsequent messages are ignored.
pub struct OnceListener<'a, T>
where
    T: Send + 'a,
{
    inner: Listener<'a, T>,
}

impl<T> OnceListener<'static, T>
where
    T: Send + 'static,
{
    /// Spawns a `OnceListener` that takes a message from the given `receiver` and executes the given `handler`.
    pub fn spawn(receiver: Receiver<T>, handler: impl FnOnce(T) -> () + Send + 'static) -> Self {
        let handler_cell = SingleUseCell::new(handler);
        let inner = Listener::spawn(receiver, move |msg| {
            handler_cell.take().unwrap()(msg);
            StopProcessing
        });
        Self { inner }
    }
}

impl<'a, T> OnceListener<'a, T>
where
    T: Send + 'a,
{
    /// Spawns a `OnceListener` that takes a message from the given `receiver` and executes the given `handler`.
    ///
    /// This is bound to the lifetime of the given scope.
    pub fn spawn_scoped<'env: 'a>(
        scope: &'a Scope<'a, 'env>,
        receiver: Receiver<T>,
        handler: impl FnOnce(T) -> () + Send + 'a,
    ) -> Self {
        let handler_cell = SingleUseCell::new(handler);
        let inner = Listener::spawn_scoped(scope, receiver, move |msg| {
            handler_cell.take().unwrap()(msg);
            StopProcessing
        });
        Self { inner }
    }

    /// Stops the `OnceListener` from processing a message, if it isn't/hasn't already.
    ///
    /// This is processed asynchronously, so if a message comes in at nearly the same time as this request, behavior is indeterminate.
    pub fn stop(&self) {
        self.inner.stop();
    }

    /// Returns `true` if and only if the `OnceListener` is waiting to process or is processing a message.
    pub fn active(&self) -> bool {
        self.inner.active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::mailbox::Mailbox;
    use crate::test_util::test_util::test_util::wait_for_condition;
    use crossbeam_channel::unbounded;
    use std::ops::{Deref, DerefMut};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    fn wait_for_listener_death<'a, T: Send + 'a>(listener: &OnceListener<'a, T>) {
        wait_for_condition(
            || !listener.active(),
            Duration::from_secs(1),
            "Listener never died.",
        );
    }

    #[test]
    fn test_once_listener() {
        let (s, r) = unbounded();
        let mtx = Arc::new(Mutex::new(1));

        let mtx_clone = mtx.clone();
        let l = OnceListener::spawn(r, move |msg| {
            *mtx_clone.lock().unwrap().deref_mut() = msg;
        });

        assert!(l.active());
        s.send_msg(2);
        s.send_msg(3);
        wait_for_listener_death(&l);

        assert_eq!(mtx.lock().unwrap().deref(), &2);
    }

    #[test]
    fn test_once_listener_stop() {
        let (_s, r) = unbounded();

        let l = OnceListener::spawn(r, |_: ()| {
            panic!("Should not run OnceListener body.");
        });

        assert!(l.active());
        l.stop();
        wait_for_listener_death(&l);
    }

    #[test]
    fn test_once_listener_scoped() {
        let (s, r) = unbounded();
        let mtx = Arc::new(Mutex::new(1));

        thread::scope(|scope| {
            let mtx_clone = mtx.clone();
            let l = OnceListener::spawn_scoped(scope, r, move |msg| {
                *mtx_clone.lock().unwrap().deref_mut() = msg;
            });

            assert!(l.active());
            s.send_msg(2);
            s.send_msg(3);
            wait_for_listener_death(&l);
        });

        assert_eq!(mtx.lock().unwrap().deref(), &2);
    }

    #[test]
    fn test_once_listener_scoped_stop() {
        let (_s, r) = unbounded();

        thread::scope(|scope| {
            let l = OnceListener::spawn_scoped(scope, r, |_: ()| {
                panic!("Should not run OnceListener body.");
            });

            assert!(l.active());
            l.stop();
            wait_for_listener_death(&l);
        })
    }
}
