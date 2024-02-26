use crate::threading::actor::Actor;
use crate::threading::actor::ActorBehavior::StopProcessing;
use crate::threading::mailbox::Mailbox;
use crate::threading::once_mailbox::OnceMailbox;
use crate::threading::single_use_cell::SingleUseCell;
use std::thread::Scope;

/// Waits for a message, then runs a function in response to it.
///
/// All subsequent messages are ignored.
pub struct OnceActor<'a, T>
where
    T: Send + 'a,
{
    mailbox: OnceMailbox<'a, T>,
    inner: Actor<'a, T>,
}

impl<T> OnceActor<'static, T>
where
    T: Send + 'static,
{
    pub fn spawn(handler: impl FnOnce(T) -> () + Send + 'static) -> Self {
        let handler_cell = SingleUseCell::new(handler);
        let inner = Actor::spawn(move |msg| {
            handler_cell.take().unwrap()(msg);
            StopProcessing
        });
        let mailbox = OnceMailbox::wrap(inner.mailbox());
        Self { mailbox, inner }
    }
}

impl<'a, T> OnceActor<'a, T>
where
    T: Send + 'a,
{
    pub fn spawn_scoped<'env: 'a>(
        scope: &'a Scope<'a, 'env>,
        handler: impl FnOnce(T) -> () + Send + 'a,
    ) -> Self {
        let handler_cell = SingleUseCell::new(handler);
        let inner = Actor::spawn_scoped(scope, move |msg| {
            handler_cell.take().unwrap()(msg);
            StopProcessing
        });
        let mailbox = OnceMailbox::wrap(inner.mailbox());
        Self { mailbox, inner }
    }

    pub fn mailbox(&self) -> impl Mailbox<'a, Message = T> + 'a {
        self.mailbox.clone()
    }

    pub fn assassinate(&self) {
        self.inner.assassinate();
    }
}

impl<'a, T: Send + 'a> Mailbox<'a> for OnceActor<'a, T> {
    type Message = T;

    fn give_message(&self, msg: Self::Message) -> bool {
        self.mailbox().give_message(msg)
    }
}
