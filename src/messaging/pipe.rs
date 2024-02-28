use crate::messaging::fan::Fan;
use crate::messaging::listener::Listener;
use crate::messaging::mailbox::Mailbox;
use crossbeam_channel::Receiver;
use std::thread::Scope;

pub struct Pipe<'a, T>
where
    T: Send + 'a,
{
    listener: Listener<'a, T>,
}

impl<T> Pipe<'static, T>
where
    T: Send + 'static,
{
    pub fn new(
        receiver: Receiver<T>,
        mailbox: impl Mailbox<'static, Message = T> + 'static,
    ) -> Self {
        Self {
            listener: Listener::spawn(receiver, move |msg| {
                mailbox.send_msg(msg);
            }),
        }
    }
}

impl<'a, T> Pipe<'a, T>
where
    T: Send + 'a,
{
    pub fn new_scoped<'env: 'a, M>(
        scope: &'a Scope<'a, 'env>,
        receiver: Receiver<T>,
        mailbox: impl Mailbox<'a, Message = T> + 'a,
    ) -> Self {
        Self {
            listener: Listener::spawn_scoped(scope, receiver, move |msg| {
                mailbox.send_msg(msg);
            }),
        }
    }
}
