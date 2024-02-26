use crate::threading::actor::ActorBehavior::{ContinueProcessing, StopProcessing};
use crate::threading::actor::MaybeScopedJoinHandle::{NotScoped, Scoped};
use crate::threading::mailbox::Mailbox;
use crossbeam_channel::{bounded, select, unbounded, Sender};
use std::thread;
use std::thread::{JoinHandle, Scope, ScopedJoinHandle};

/// Determines what the `Actor` should do after processing the current message.
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub enum ActorBehavior {
    #[default]
    ContinueProcessing,
    StopProcessing,
}

impl Into<ActorBehavior> for () {
    fn into(self) -> ActorBehavior {
        ContinueProcessing
    }
}

enum MaybeScopedJoinHandle<'a, T> {
    NotScoped(JoinHandle<T>),
    Scoped(ScopedJoinHandle<'a, T>),
}

/// A thread that continually reads messages from its `Mailbox` and executes the given function on them.
pub struct Actor<'a, T>
where
    T: Send + 'a,
{
    mailbox: Sender<T>,
    thread: MaybeScopedJoinHandle<'a, ()>,
    cancel_sender: Sender<()>,
}

impl<T> Actor<'static, T>
where
    T: Send + 'static,
{
    /// Executes the given payload for each message that enters its `Mailbox`.
    ///
    /// Terminates on `cancel()`, if receiving fails, if the payload returns `StopProcessing`, or when the `Actor` is dropped.
    pub fn spawn<B, FPayload>(payload: FPayload) -> Self
    where
        B: Into<ActorBehavior>,
        FPayload: Fn(T) -> B + Send + 'static,
    {
        let (mailbox, receiver) = unbounded();
        let (cancel_sender, cancel_receiver) = bounded(1);

        let thread = NotScoped(thread::spawn(move || loop {
            select! {
                recv(cancel_receiver) -> _ => return,
                recv(receiver) -> val => {
                    if let Ok(v) = val {
                        if payload(v).into() == StopProcessing {
                            return;
                        }
                    } else {
                        return;
                    }
                }
            }
        }));

        Self {
            mailbox,
            cancel_sender,
            thread,
        }
    }
}

impl<'a, T> Actor<'a, T>
where
    T: Send + 'a,
{
    /// Executes the given payload for each message that enters its `Mailbox`.
    ///
    /// Tied to the lifetime of the `Scope` used to create it.
    ///
    /// Terminates on `cancel()`, if receiving fails, if the payload returns `StopProcessing`, or when the `Actor` is dropped.
    pub fn spawn_scoped<'env, B, FPayload>(scope: &'a Scope<'a, 'env>, payload: FPayload) -> Self
    where
        'env: 'a,
        B: Into<ActorBehavior>,
        FPayload: Fn(T) -> B + Send + 'a,
    {
        let (mailbox, receiver) = unbounded();
        let (cancel_sender, cancel_receiver) = bounded(1);

        let thread = Scoped(scope.spawn(move || loop {
            select! {
                recv(cancel_receiver) -> _ => return,
                recv(receiver) -> val => {
                    if let Ok(v) = val {
                        if payload(v).into() == StopProcessing {
                            return;
                        }
                    } else {
                        return;
                    }
                }
            }
        }));

        Self {
            mailbox,
            cancel_sender,
            thread,
        }
    }

    /// Gets the mailbox of this actor, meaning the place where you can send messages to it.
    pub fn mailbox(&self) -> impl Mailbox<'a, Message = T> {
        self.mailbox.clone()
    }

    /// Stops the actor, preventing it from processing further messages.
    ///
    /// The execution waits until the actor has processed its current message, if it's currently processing one.
    pub fn assassinate(&self) {
        let _ = self.cancel_sender.try_send(());
    }

    /// `true` if the Actor is still processing messages.
    pub fn alive(&self) -> bool {
        match &self.thread {
            NotScoped(h) => h.is_finished(),
            Scoped(h) => h.is_finished(),
        }
    }
}

impl<'a, T> Drop for Actor<'a, T>
where
    T: Send + 'a,
{
    fn drop(&mut self) {
        self.assassinate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::collect_collection::CollectVec;
    use crate::test_util::test_util::test_util::wait_for_condition;
    use std::ops::Deref;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn test_actor() {
        let processed = Arc::new(Mutex::new(Vec::new()));

        let processed_clone = processed.clone();
        let actor = Actor::spawn(move |num| {
            processed_clone.lock().unwrap().push(num);
            if num == 5 {
                StopProcessing
            } else {
                ContinueProcessing
            }
        });

        for i in 0..7 {
            actor.mailbox.give_message(i);
        }

        wait_for_condition(
            || !actor.alive(),
            Duration::from_secs(1),
            "Actor never died.",
        );

        assert_eq!(processed.lock().unwrap().deref(), &(0..=5).collect_vec());
    }

    #[test]
    fn test_actor_cancel_terminates() {
        let actor = Arc::new(Actor::spawn(|_| ContinueProcessing));

        let actor_clone = actor.clone();
        thread::spawn(move || loop {
            while actor_clone.alive() {
                actor_clone.mailbox().give_message(());
            }
        });

        assert!(actor.alive());
        actor.assassinate();
        wait_for_condition(
            || !actor.alive(),
            Duration::from_secs(1),
            "Actor never died.",
        );
    }

    #[test]
    fn test_scoped_actor() {
        let processed = Arc::new(Mutex::new(Vec::new()));

        let processed_clone = processed.clone();
        thread::scope(move |scope| {
            let actor = Actor::spawn_scoped(scope, |num| {
                processed_clone.lock().unwrap().push(num);

                if num == 5 {
                    StopProcessing
                } else {
                    ContinueProcessing
                }
            });

            for i in 0..7 {
                actor.mailbox().give_message(i);
            }

            wait_for_condition(
                || !actor.alive(),
                Duration::from_secs(1),
                "Actor never died.",
            );
        });

        assert_eq!(processed.lock().unwrap().deref(), &(0..=5).collect_vec());
    }

    #[test]
    fn test_scoped_background_cancel_terminates() {
        let (send, recv) = unbounded();

        thread::scope(|scope| {
            let actor = Actor::spawn_scoped(scope, |_| ContinueProcessing);

            scope.spawn(|| loop {
                match send.send(()) {
                    Ok(_) => {}
                    Err(_) => return,
                };
            });

            assert!(actor.alive());
            actor.assassinate();
            wait_for_condition(
                || !actor.alive(),
                Duration::from_secs(1),
                "Actor never died.",
            );
        });
    }
}
