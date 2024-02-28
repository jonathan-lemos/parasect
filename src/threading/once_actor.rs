use crate::messaging::listener::ListenerBehavior;
use crate::messaging::mailbox::Mailbox;
use crate::messaging::once_listener::OnceListener;
use crate::threading::single_use_cell::SingleUseCell;
use crossbeam_channel::{unbounded, Sender};
use std::thread::Scope;

pub struct OnceActor<'a, T>
where
    T: Send + 'a,
{
    listener: OnceListener<'a, T>,
    mailbox: Sender<T>,
}

fn closure<'a, T: Send + 'a>(payload: impl FnOnce(T) + Send + 'a) -> impl Fn(T) + Send + 'a {
    let payload_cell = SingleUseCell::new(payload);
    move |msg| {
        payload_cell.take().unwrap()(msg);
    }
}

impl<T> OnceActor<'static, T>
where
    T: Send + 'static,
{
    pub fn spawn(payload: impl FnOnce(T) + Send + 'static) -> Self {
        let (send, recv) = unbounded();
        Self {
            listener: OnceListener::spawn(recv, closure(payload)),
            mailbox: send,
        }
    }
}

impl<'a, T> OnceActor<'a, T>
where
    T: Send + 'a,
{
    pub fn spawn_scoped<'env: 'a>(
        scope: &'a Scope<'a, 'env>,
        payload: impl Fn(T) + Send + 'a,
    ) -> Self {
        let (send, recv) = unbounded();
        Self {
            listener: OnceListener::spawn_scoped(scope, recv, closure(payload)),
            mailbox: send,
        }
    }

    pub fn mailbox(&self) -> impl Mailbox<'a, Message = T> {
        self.mailbox.clone()
    }

    pub fn stop(&self) {
        self.listener.stop()
    }
}
