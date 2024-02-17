use crate::task::cancellable_message::ValueState::*;
use crate::task::cancellable_task::CancellableTask;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::cell::UnsafeCell;
use std::sync::{Arc, RwLock};

enum ValueState<T: Send> {
    Unset,
    Cancelled,
    Set(T),
}

/// Asynchronously sends a single T that may be cancelled.
///
/// Any send() or cancel() after the first of either will be ignored.
/// Messages can be sent and received in any thread.
/// CancellableMessages can be cloned, in which case any receiver will get the same value.
pub struct CancellableMessage<T: Send + Sync> {
    sender: Sender<Option<T>>,
    receiver: Receiver<Option<T>>,
    // We need unsafe anyway to return a shared reference to the held data.
    // Trying to do so with e.g. RefCell's .borrow() runs into the problem that the reference is
    // tied to the lifetime of the RwLockReadGuard, so we can't return the reference.
    // Doing so should be safe because we only mutate the cell once, and that mutation happens
    // before any shared references are returned, so the data should be stable once a shared
    // reference is returned.
    //
    // Because we need unsafe to use this anyway, there's no reason not to use UnsafeCell.
    inner_value: Arc<RwLock<UnsafeCell<ValueState<T>>>>,
}

impl<T: Send + Sync> CancellableMessage<T> {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(1);
        Self {
            sender,
            receiver,
            inner_value: Arc::new(RwLock::new(UnsafeCell::new(Unset))),
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
        {
            let value_read = self.inner_value.read().unwrap();

            // unsafe is required to get a shared reference to the ValueState behind the RwLock that
            // is *not tied to the lifetime of the guard*
            let immut_ref = unsafe { value_read.get().as_ref().unwrap() };

            match immut_ref {
                Set(v) => return Some(v),
                Cancelled => return None,
                Unset => {}
            }
        }
        {
            let mut value_write = self.inner_value.write().unwrap();

            let mut_ref = value_write.get_mut();

            // have to check again in case two threads both try to write
            match mut_ref {
                // once again, unsafe is required to get a reference that is not tied to the
                // lifetime of the guard
                Set(v) => return Some(unsafe { (v as *const T).as_ref().unwrap() }),
                Cancelled => return None,
                Unset => {}
            }

            *mut_ref = match self.receiver.recv().unwrap() {
                Some(v) => Set(v),
                None => Cancelled,
            };

            match mut_ref {
                Set(v) => return Some(unsafe { (v as *const T).as_ref().unwrap() }),
                Cancelled => return None,
                Unset => {}
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

    fn join(&self) -> Option<&T> {
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
    use crate::task::test_util::test_util::assert_result_eq;
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
                let result = scope.spawn(|| cm.recv().map(|x| *x));
                scope.spawn(|| cm.send(69));

                result.join().unwrap()
            })
        };

        if let None = answer {
            panic!("answer should not be None");
        }

        assert_result_eq!(answer, 69);
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
