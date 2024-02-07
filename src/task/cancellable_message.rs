use crate::task::cancellable_task::CancellableTask;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::cell::OnceCell;
use std::sync::{Arc, RwLock};

/// Asynchronously sends a single T that may be cancelled.
///
/// Any send() or cancel() after the first of either will be ignored.
/// Messages can be sent and received in any thread.
/// CancellableMessages can be cloned, in which case any receiver will get the same value.
pub struct CancellableMessage<T: Send + Sync> {
    sender: Sender<Option<Arc<T>>>,
    receiver: Receiver<Option<Arc<T>>>,
    inner_value: Arc<RwLock<OnceCell<Option<Arc<T>>>>>,
}

impl<T: Send + Sync> CancellableMessage<T> {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(1);
        Self {
            sender,
            receiver,
            inner_value: Arc::new(RwLock::new(OnceCell::new())),
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
        let _ = self.sender.send(None);
    }

    /// Send a value.
    /// All future cancel() or send() calls will be ignored.
    pub fn send<ArcLike: Into<Arc<T>>>(&self, value: ArcLike) -> () {
        // same logic as cancel() for why we can ignore the result
        let _ = self.sender.send(Some(value.into()));
    }

    /// Receives a reference to a value (Some) or a cancellation (None).
    /// Blocks until send() or cancel() is called.
    ///
    /// Repeated calls to this function will return the same value as the first call.
    pub fn recv(&self) -> Option<Arc<T>> {
        {
            let value_read = self.inner_value.read().unwrap();

            if let Some(v) = value_read.get() {
                return v.clone();
            }
        }
        {
            let value_write = self.inner_value.write().unwrap();

            // have to check again in case two threads both try to write
            if let Some(v) = value_write.get() {
                return v.clone();
            }

            if let Err(_) = value_write.set(self.receiver.recv().unwrap()) {
                panic!("tried to set OnceCell twice. this should never happen");
            }

            if let Some(v) = value_write.get() {
                return v.clone();
            }

            panic!("should never happen")
        }
    }
}

impl<T: Send + Sync> Clone for CancellableMessage<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            inner_value: self.inner_value.clone(),
        }
    }
}

impl<T: Send + Sync> CancellableTask<T> for CancellableMessage<T> {
    fn request_cancellation(&self) -> () {
        self.cancel();
    }

    fn join(&self) -> Option<Arc<T>> {
        self.recv()
    }
}

// The OnceCell cannot be mutated without an exclusive write lock.
// Therefore, we claim that this is thread-safe.
unsafe impl<T> Send for CancellableMessage<T> where T: Send + Sync {}
unsafe impl<T> Sync for CancellableMessage<T> where T: Send + Sync {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_result_eq;
    use crate::task::test_util::test_util::*;
    use crate::test_util::test_util::test_util::detect_flake;
    use std::thread;

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
    fn ensure_exclusive_arc() {
        let answer = {
            let cm = CancellableMessage::<i64>::new();

            thread::scope(|scope| {
                let result = scope.spawn(|| cm.recv());
                scope.spawn(|| cm.send(69));

                result.join().unwrap()
            })
        };

        if let None = answer {
            panic!("answer should not be None");
        }

        assert_eq!(Arc::into_inner(answer.unwrap()), Some(69));
    }

    #[test]
    fn test_multiple_recv() {
        let (answer1, answer2, answer3) = {
            let cm = CancellableMessage::<i64>::new();

            thread::scope(|scope| {
                let a1 = scope.spawn(|| cm.recv());
                let a2 = scope.spawn(|| cm.recv());
                let a3 = scope.spawn(|| cm.recv());

                scope.spawn(|| cm.send(69));

                (a1.join().unwrap(), a2.join().unwrap(), a3.join().unwrap())
            })
        };

        assert_eq!(answer1, answer2);
        assert_eq!(answer2, answer3);
        assert_eq!(answer1, Some(Arc::new(69)));
    }

    #[test]
    fn test_detect_flakes() {
        detect_flake(|| {
            let (answer1, answer2, answer3, answer4) = {
                let cm = CancellableMessage::<i64>::new();

                thread::scope(|scope| {
                    let a1 = scope.spawn(|| cm.recv());
                    let a2 = scope.spawn(|| cm.recv());
                    let a3 = scope.spawn(|| cm.recv());
                    let a4 = scope.spawn(|| cm.recv());

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
            assert_eq!(answer1, Some(Arc::new(69)));
        })
    }

    #[test]
    fn test_clone() {
        let cm = CancellableMessage::new();
        let cm2 = cm.clone();

        cm2.send(69);

        assert_result_eq!(cm2.recv(), 69);
        assert_eq!(cm.recv(), cm2.recv());
    }

    #[test]
    fn test_clone_2() {
        let cm = CancellableMessage::new();
        let cm2 = cm.clone();

        cm.send(69);

        assert_eq!(cm.recv(), cm2.recv());
        assert_result_eq!(cm.recv(), 69);
    }
}
