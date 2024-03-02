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
    pub fn spawn<B: Into<ListenerBehavior> + 'static>(
        payload: impl FnMut(T) -> B + Send + 'static,
    ) -> Self {
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
    pub fn spawn_scoped<'env: 'a, B: Into<ListenerBehavior> + 'a>(
        scope: &'a Scope<'a, 'env>,
        payload: impl FnMut(T) -> B + Send + 'a,
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

    pub fn stop(&self) {
        self.listener.stop();
    }

    pub fn active(&self) -> bool {
        self.listener.active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::listener::ListenerBehavior::{ContinueProcessing, StopProcessing};
    use crate::test_util::test_util::test_util::wait_for_condition;
    use std::ops::Deref;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    fn wait_for_actor_death<'a, T: Send + 'a>(actor: &Actor<'a, T>) {
        wait_for_condition(
            || !actor.active(),
            Duration::from_secs(1),
            "Actor never died.",
        );
    }

    #[test]
    fn test_actor() {
        let vec = Arc::new(Mutex::new(Vec::new()));

        let vec_clone = vec.clone();
        let actor = Actor::spawn(move |msg| {
            vec_clone.lock().unwrap().push(msg);
            if msg == 5 {
                StopProcessing
            } else {
                ContinueProcessing
            }
        });

        assert!(actor.active());

        for i in 0..7 {
            actor.mailbox().send_msg(i);
        }

        wait_for_actor_death(&actor);

        assert_eq!(vec.lock().unwrap().deref(), &vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_actor_stop() {
        let actor = Arc::new(Actor::spawn(|_| {}));

        let actor_clone = actor.clone();
        thread::spawn(move || {
            while actor_clone.active() {
                actor_clone.mailbox().send_msg(());
            }
        });

        assert!(actor.active());
        actor.stop();
        wait_for_actor_death(&actor);
    }

    #[test]
    fn test_actor_scoped() {
        let vec = Arc::new(Mutex::new(Vec::new()));

        let vec_clone = vec.clone();
        thread::scope(|scope| {
            let actor = Actor::spawn_scoped(scope, move |msg| {
                vec_clone.lock().unwrap().push(msg);
                if msg == 5 {
                    StopProcessing
                } else {
                    ContinueProcessing
                }
            });

            assert!(actor.active());

            for i in 0..7 {
                actor.mailbox().send_msg(i);
            }

            wait_for_actor_death(&actor);
        });

        assert_eq!(vec.lock().unwrap().deref(), &vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_actor_scoped_stop() {
        thread::scope(|scope| {
            let actor = Arc::new(Actor::spawn_scoped(scope, |_| {}));

            let actor_clone = actor.clone();
            scope.spawn(move || {
                while actor_clone.active() {
                    actor_clone.mailbox().send_msg(());
                }
            });

            assert!(actor.active());
            actor.stop();
            wait_for_actor_death(&actor);
        })
    }
}
