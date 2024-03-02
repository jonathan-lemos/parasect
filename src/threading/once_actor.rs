use crate::messaging::mailbox::Mailbox;
use crate::messaging::once_listener::OnceListener;
use crate::messaging::once_mailbox::OnceMailbox;
use crate::threading::single_use_cell::SingleUseCell;
use crossbeam_channel::Sender;
use std::thread::Scope;

pub struct OnceActor<'a, T>
where
    T: Send + 'a,
{
    listener: OnceListener<'a, T>,
    mailbox: OnceMailbox<'a, T, Sender<T>>,
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
        let (mailbox, recv) = OnceMailbox::new();
        Self {
            listener: OnceListener::spawn(recv, closure(payload)),
            mailbox,
        }
    }
}

impl<'a, T> OnceActor<'a, T>
where
    T: Send + 'a,
{
    #[allow(unused)]
    pub fn spawn_scoped<'env: 'a>(
        scope: &'a Scope<'a, 'env>,
        payload: impl Fn(T) + Send + 'a,
    ) -> Self {
        let (send, recv) = OnceMailbox::new();
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

    #[allow(unused)]
    pub fn active(&self) -> bool {
        self.listener.active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_util::test_util::wait_for_condition;
    use std::ops::{Deref, DerefMut};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    fn wait_for_actor_death<'a, T: Send + 'a>(actor: &OnceActor<'a, T>) {
        wait_for_condition(
            || !actor.active(),
            Duration::from_secs(1),
            "Actor never died.",
        );
    }

    #[test]
    fn test_onceactor() {
        let val = Arc::new(Mutex::new(1));

        let val_clone = val.clone();
        let actor = OnceActor::spawn(move |msg| {
            *val_clone.lock().unwrap().deref_mut() = msg;
        });

        assert!(actor.active());
        assert!(actor.mailbox().send_msg(2));
        assert!(!actor.mailbox().send_msg(3));
        wait_for_actor_death(&actor);

        assert_eq!(val.lock().unwrap().deref(), &2);
    }

    #[test]
    fn test_onceactor_stop() {
        let actor = Arc::new(OnceActor::spawn(|_: ()| {}));

        assert!(actor.active());
        actor.stop();
        wait_for_actor_death(&actor);
    }

    #[test]
    fn test_onceactor_scoped() {
        let val = Arc::new(Mutex::new(1));

        let val_clone = val.clone();
        thread::scope(|scope| {
            let actor = OnceActor::spawn_scoped(scope, move |msg| {
                *val_clone.lock().unwrap().deref_mut() = msg;
            });

            assert!(actor.active());
            assert!(actor.mailbox().send_msg(2));
            assert!(!actor.mailbox().send_msg(3));
            wait_for_actor_death(&actor);
        });

        assert_eq!(val.lock().unwrap().deref(), &2);
    }

    #[test]
    fn test_actor_scoped_stop() {
        thread::scope(|scope| {
            let actor = Arc::new(OnceActor::spawn_scoped(scope, |_: ()| {}));

            assert!(actor.active());
            actor.stop();
            wait_for_actor_death(&actor);
        })
    }
}
