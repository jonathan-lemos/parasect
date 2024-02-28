use crate::messaging::listener::{Listener, ListenerBehavior};
use crate::messaging::mailbox::Mailbox;
use crossbeam_channel::{unbounded, Sender};
use std::thread::Scope;

pub struct Actor<'a, T>
where
    T: Send + 'a,
{
    listener: Listener<'a, T>,
    mailbox: Sender<T>,
}

impl<T> Actor<'static, T>
where
    T: Send + 'static,
{
    pub fn spawn(payload: impl Fn(T) -> ListenerBehavior + Send + 'static) -> Self {
        let (send, recv) = unbounded();
        Self {
            listener: Listener::spawn(recv, payload),
            mailbox: send,
        }
    }
}

impl<'a, T> Actor<'a, T>
where
    T: Send + 'a,
{
    pub fn spawn_scoped<'env: 'a>(
        scope: &'a Scope<'a, 'env>,
        payload: impl Fn(T) -> ListenerBehavior + Send + 'a,
    ) -> Self {
        let (send, recv) = unbounded();
        Self {
            listener: Listener::spawn_scoped(scope, recv, payload),
            mailbox: send,
        }
    }

    pub fn mailbox(&self) -> impl Mailbox<'a, Message = T> {
        self.mailbox.clone()
    }
}
