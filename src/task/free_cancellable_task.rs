use crate::messaging::mailbox::Mailbox;
use crate::task::cancellable_task::CancellableTask;
use std::sync::atomic::{AtomicBool, Ordering};

/// Wraps a value in the CancellableTask trait.
///
/// cancel() before notify() or wait() will return None instead of the given value.
pub struct FreeCancellableTask<T>
where
    T: Send + Sync + Clone + 'static,
{
    value: T,
    cancelled: AtomicBool,
    value_was_returned: AtomicBool,
}

impl<T> CancellableTask<T> for FreeCancellableTask<T>
where
    T: Send + Sync + Clone + 'static,
{
    fn notify_when_done(&self, notifiable: impl Mailbox<'static, Message = Option<T>> + 'static) {
        notifiable.send_msg(
            if self.cancelled.load(Ordering::Relaxed)
                && !self.value_was_returned.load(Ordering::Relaxed)
            {
                None
            } else {
                self.value_was_returned.store(true, Ordering::Relaxed);
                Some(self.value.clone())
            },
        );
    }

    fn request_cancellation(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

#[allow(unused)]
impl<T> FreeCancellableTask<T>
where
    T: Send + Sync + Clone + 'static,
{
    /// Creates a CancellableTask out of a T.
    pub fn new(value: T) -> Self {
        Self {
            value,
            cancelled: AtomicBool::new(false),
            value_was_returned: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::test_util::*;
    use crossbeam_channel::bounded;
    use proptest::prelude::*;

    #[test]
    fn notify_returns_value() {
        let task = FreeCancellableTask::<i64>::new(69);
        let (s, r) = bounded(1);

        task.notify_when_done(s);
        assert_result_eq!(r.recv().unwrap(), 69);
    }

    #[test]
    fn notify_returns_none_on_cancel() {
        let task = FreeCancellableTask::<i64>::new(69);
        let (s, r) = bounded(1);

        task.request_cancellation();
        task.notify_when_done(s);

        assert_eq!(r.recv().unwrap(), None);
    }

    #[test]
    fn wait_returns_value() {
        let task = FreeCancellableTask::<i64>::new(69);
        assert_result_eq!(task.wait(), 69);
    }

    #[test]
    fn wait_returns_none_on_cancel() {
        let task = FreeCancellableTask::<i64>::new(69);
        task.request_cancellation();
        assert_eq!(task.wait(), None);
    }

    #[test]
    fn test_assert_ct_invariants() {
        assert_cancellabletask_invariants(|| FreeCancellableTask::<i64>::new(69));
    }

    proptest! {
        #[test]
        fn test_threadsafe(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| FreeCancellableTask::<i32>::new(i));
        }
    }
}
