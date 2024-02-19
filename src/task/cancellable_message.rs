use crate::task::cancellable_task::CancellableTask;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::OnceLock;

/// Asynchronously sends a single T that may be cancelled.
///
/// Any send() or cancel() after the first of either will be ignored.
/// Messages can be sent and received in any thread.
pub struct CancellableMessage<T: Send + Sync> {
    sender: Sender<Option<T>>,
    receiver: Receiver<Option<T>>,
    inner_value: OnceLock<Option<T>>,
}

impl<T: Send + Sync> CancellableMessage<T> {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(1);
        Self {
            sender,
            receiver,
            inner_value: OnceLock::new(),
        }
    }

    /// Cancel the message.
    /// All future cancel() or send() calls will be ignored.
    pub fn cancel(&self) -> () {
        // if this fails to send, it's because
        // 1) there is already a message in the channel, or
        // 2) the channel is disconnected
        //
        // in both cases, we don't care if this message is dropped,
        // so it's unnecessary to handle it
        let _ = self.sender.try_send(None);
    }

    /// Send a value.
    /// All future cancel() or send() calls will be ignored.
    pub fn send(&self, value: T) -> () {
        // same logic as cancel() for why we can ignore the result
        let _ = self.sender.try_send(Some(value));
    }

    /// Receives a reference to a value (Some) or a cancellation (None).
    /// Blocks until send() or cancel() is called.
    ///
    /// Repeated calls to this function will return the same value as the first call.
    pub fn recv(&self) -> Option<&T> {
        self.inner_value
            .get_or_init(|| self.receiver.recv().unwrap())
            .as_ref()
    }

    /// Receives a value (Some) or a cancellation (None).
    /// Blocks until send() or cancel() is called.
    pub fn recv_into(mut self) -> Option<T> {
        self.recv();
        self.inner_value.take().unwrap()
    }
}

impl<T: Send + Sync> CancellableTask<T> for CancellableMessage<T> {
    fn join(&self) -> Option<&T> {
        self.recv()
    }

    fn join_into(self) -> Option<T> {
        self.recv_into()
    }

    fn request_cancellation(&self) -> () {
        self.cancel();
    }
}

// The OnceCell cannot be mutated without an exclusive write lock.
// Therefore, we claim that this is thread-safe.
unsafe impl<T> Send for CancellableMessage<T> where T: Send + Sync {}

unsafe impl<T> Sync for CancellableMessage<T> where T: Send + Sync {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::test_util::*;
    use crate::test_util::test_util::test_util::detect_flake;
    use std::thread;

    #[test]
    fn test_recv_into() {
        let cm = CancellableMessage::<i64>::new();

        cm.send(69);
        assert_result_eq!(cm.recv_into(), 69);
    }

    #[test]
    fn test_send_recv() {
        let cm = CancellableMessage::<i64>::new();

        let answer = thread::scope(|scope| {
            let result = scope.spawn(|| cm.recv());
            scope.spawn(|| cm.send(69));

            result.join().unwrap()
        });

        assert_result_eq!(answer, 69);
    }

    #[test]
    fn test_multiple_sends_returns_first() {
        let cm = CancellableMessage::<i64>::new();

        let answer1 = thread::scope(|scope| {
            let result = scope.spawn(|| cm.recv());
            scope.spawn(|| cm.send(69));

            result.join().unwrap()
        });

        let answer2 = thread::scope(|scope| {
            let result = scope.spawn(|| cm.recv());
            scope.spawn(|| cm.send(70));

            result.join().unwrap()
        });

        assert_result_eq!(answer1, 69);
        assert_result_eq!(answer2, 69);
    }

    #[test]
    fn test_cancel() {
        let cm = CancellableMessage::<i64>::new();

        let answer = thread::scope(|scope| {
            let result = scope.spawn(|| cm.recv());
            scope.spawn(|| cm.cancel());

            result.join().unwrap()
        });

        assert_eq!(answer, None);
    }

    #[test]
    fn cancel_idempotent() {
        let cm = CancellableMessage::<i64>::new();

        let answer = thread::scope(|scope| {
            let result = scope.spawn(|| cm.recv());
            scope.spawn(|| cm.cancel());
            scope.spawn(|| cm.cancel());

            result.join().unwrap()
        });

        assert_eq!(answer, None);
    }

    #[test]
    fn cancel_keeps_returning_none() {
        let cm = CancellableMessage::<i64>::new();

        let answer = thread::scope(|scope| {
            let result = scope.spawn(|| cm.recv());
            scope.spawn(|| cm.cancel());

            result.join().unwrap()
        });

        assert_eq!(answer, None);
        assert_eq!(cm.recv(), None);
    }

    #[test]
    fn test_multiple_recv() {
        let (answer1, answer2, answer3) = {
            let cm = CancellableMessage::<i64>::new();

            thread::scope(|scope| {
                let a1 = scope.spawn(|| cm.recv().map(|x| *x));
                let a2 = scope.spawn(|| cm.recv().map(|x| *x));
                let a3 = scope.spawn(|| cm.recv().map(|x| *x));

                scope.spawn(|| cm.send(69));

                (a1.join().unwrap(), a2.join().unwrap(), a3.join().unwrap())
            })
        };

        assert_eq!(answer1, answer2);
        assert_eq!(answer2, answer3);
        assert_result_eq!(answer1, 69);
    }

    #[test]
    fn test_detect_flakes() {
        detect_flake(|| {
            let (answer1, answer2, answer3, answer4) = {
                let cm = CancellableMessage::<i64>::new();

                thread::scope(|scope| {
                    let a1 = scope.spawn(|| cm.recv().map(|x| *x));
                    let a2 = scope.spawn(|| cm.recv().map(|x| *x));
                    let a3 = scope.spawn(|| cm.recv().map(|x| *x));
                    let a4 = scope.spawn(|| cm.recv().map(|x| *x));

                    scope.spawn(|| cm.send(69));
                    scope.spawn(|| cm.send(69));

                    (
                        a1.join().unwrap(),
                        a2.join().unwrap(),
                        a3.join().unwrap(),
                        a4.join().unwrap(),
                    )
                })
            };

            assert_eq!(answer1, answer2);
            assert_eq!(answer2, answer3);
            assert_eq!(answer3, answer4);
            assert_result_eq!(answer1, 69);
        })
    }

    #[test]
    fn test_ct_invariants() {
        assert_cancellabletask_invariants(|| {
            let cm = CancellableMessage::<i64>::new();
            cm.send(69);
            cm
        })
    }
}
