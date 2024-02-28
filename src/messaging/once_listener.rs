use crate::messaging::listener::Listener;
use crate::messaging::listener::ListenerBehavior::StopProcessing;
use crate::messaging::mailbox::Mailbox;
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

    pub fn stop(&self) {
        self.inner.stop();
    }

    pub fn active(&self) {
        self.inner.active();
    }
}
