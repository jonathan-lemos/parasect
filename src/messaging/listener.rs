use crate::messaging::listener::ListenerBehavior::{ContinueProcessing, StopProcessing};
use crate::messaging::listener::MaybeScopedJoinHandle::{NotScoped, Scoped};
use crate::messaging::mailbox::Mailbox;
use crossbeam_channel::{bounded, select, unbounded, Receiver, Sender};
use std::marker::PhantomData;
use std::thread;
use std::thread::{JoinHandle, Scope, ScopedJoinHandle};

/// Determines what the `Listener` should do after processing the current message.
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub enum ListenerBehavior {
    #[default]
    ContinueProcessing,
    StopProcessing,
}

impl Into<ListenerBehavior> for () {
    fn into(self) -> ListenerBehavior {
        ContinueProcessing
    }
}

enum MaybeScopedJoinHandle<'a, T> {
    NotScoped(JoinHandle<T>),
    Scoped(ScopedJoinHandle<'a, T>),
}

/// A thread that continually reads messages from the given `Receiver` and executes the given function on them.
pub struct Listener<'a, T>
where
    T: Send + 'a,
{
    _t: PhantomData<T>,
    thread: MaybeScopedJoinHandle<'a, ()>,
    cancel_sender: Sender<()>,
}

fn receiver_loop_closure<'a, B: Into<ListenerBehavior> + 'a, T: Send + 'a>(
    receiver: Receiver<T>,
    mut payload: impl FnMut(T) -> B + Send + 'a,
) -> (impl FnMut(), Sender<()>) {
    let (cancel_sender, cancel_receiver) = bounded(1);

    (
        move || loop {
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
        },
        cancel_sender,
    )
}

impl<T> Listener<'static, T>
where
    T: Send + 'static,
{
    /// Executes the given payload for each message that enters its `receiver`.
    ///
    /// Terminates on `stop()`, if receiving fails, if the payload returns `StopProcessing`, or when the `Listener` is dropped.
    pub fn spawn<B: Into<ListenerBehavior> + 'static>(
        receiver: Receiver<T>,
        payload: impl FnMut(T) -> B + Send + 'static,
    ) -> Self {
        let (closure, cancel_sender) = receiver_loop_closure(receiver, payload);

        Self {
            _t: PhantomData,
            cancel_sender,
            thread: NotScoped(thread::spawn(closure)),
        }
    }
}

impl<'a, T> Listener<'a, T>
where
    T: Send + 'a,
{
    /// Executes the given payload for each message that enters its `receiver`.
    ///
    /// Tied to the lifetime of the `Scope` used to create it.
    ///
    /// Terminates on `stop()`, if receiving fails, if the payload returns `StopProcessing`, or when the `Listener` is dropped.
    pub fn spawn_scoped<'env: 'a, B: Into<ListenerBehavior> + 'a>(
        scope: &'a Scope<'a, 'env>,
        receiver: Receiver<T>,
        payload: impl FnMut(T) -> B + Send + 'a,
    ) -> Self {
        let (closure, cancel_sender) = receiver_loop_closure(receiver, payload);

        Self {
            _t: PhantomData,
            cancel_sender,
            thread: Scoped(scope.spawn(closure)),
        }
    }

    /// Stops the listener, preventing it from processing further messages.
    ///
    /// The listener will process its current message if it's currently processing one.
    pub fn stop(&self) {
        let _ = self.cancel_sender.try_send(());
    }

    /// `true` if and only if the `Listener` is still processing messages.
    pub fn active(&self) -> bool {
        match &self.thread {
            NotScoped(h) => !h.is_finished(),
            Scoped(h) => !h.is_finished(),
        }
    }
}

impl<'a, T> Drop for Listener<'a, T>
where
    T: Send + 'a,
{
    fn drop(&mut self) {
        self.stop();
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

    fn wait_for_listener_death<'a, T: Send + 'a>(listener: &Listener<'a, T>) {
        wait_for_condition(
            || !listener.active(),
            Duration::from_secs(1),
            "Listener never died.",
        );
    }

    #[test]
    fn test_listener() {
        let (s, r) = unbounded();
        let processed = Arc::new(Mutex::new(Vec::new()));

        let processed_clone = processed.clone();

        let listener = Listener::spawn(r, move |num| {
            {
                let mut write = processed_clone.lock().unwrap();
                write.push(num);
            }
            if num == 5 {
                StopProcessing
            } else {
                ContinueProcessing
            }
        });

        for i in 0..7 {
            s.send_msg(i);
        }

        wait_for_listener_death(&listener);
        assert_eq!(processed.lock().unwrap().deref(), &(0..=5).collect_vec());
    }

    #[test]
    fn test_listener_cancel_terminates() {
        let (s, r) = unbounded();
        let listener = Arc::new(Listener::spawn(r, |_| {}));

        let listener_clone = listener.clone();
        thread::spawn(move || loop {
            while listener_clone.active() {
                s.send_msg(());
            }
        });

        assert!(listener.active());
        listener.stop();
        wait_for_listener_death(&listener);
    }

    #[test]
    fn test_scoped_listener() {
        let (s, r) = unbounded();
        let processed = Arc::new(Mutex::new(Vec::new()));

        let processed_clone = processed.clone();
        thread::scope(move |scope| {
            let listener = Listener::spawn_scoped(scope, r, move |num| {
                processed_clone.lock().unwrap().push(num);

                if num == 5 {
                    StopProcessing
                } else {
                    ContinueProcessing
                }
            });

            for i in 0..7 {
                s.send_msg(i);
            }

            wait_for_listener_death(&listener);
        });

        assert_eq!(processed.lock().unwrap().deref(), &(0..=5).collect_vec());
    }

    #[test]
    fn test_scoped_background_cancel_terminates() {
        let (send, recv) = unbounded();

        thread::scope(|scope| {
            let listener = Listener::spawn_scoped(scope, recv, |_| {});

            scope.spawn(|| loop {
                match send.send(()) {
                    Ok(_) => {}
                    Err(_) => return,
                };
            });

            assert!(listener.active());
            listener.stop();
            wait_for_listener_death(&listener);
        });
    }
}
