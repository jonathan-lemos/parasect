use crate::task::cancellable_task::CancellableTask;
use crossbeam_channel::Sender;
use std::marker::PhantomData;
use std::ops::Deref;

/// Wraps a CancellableTask, but silently drops all cancellations.
///
/// Do not instantiate directly. Call `.ignore_cancellations()` instead.
pub struct IgnoreCancelCancellableTask<T, InnerTask>
where
    T: Send + Sync + Clone + 'static,
    InnerTask: CancellableTask<T>,
{
    inner: InnerTask,
    _t: PhantomData<T>,
}

impl<T, InnerTask> CancellableTask<T> for IgnoreCancelCancellableTask<T, InnerTask>
where
    T: Send + Sync + Clone + 'static,
    InnerTask: CancellableTask<T>,
{
    fn notify_when_done(&self, sender: Sender<Option<T>>) {
        self.inner.notify_when_done(sender);
    }

    fn request_cancellation(&self) -> () {}
}

impl<T, InnerTask> IgnoreCancelCancellableTask<T, InnerTask>
where
    T: Send + Sync + Clone + 'static,
    InnerTask: CancellableTask<T>,
{
    /// Do not instantiate directly. Use `.ignore_cancellations()` on any `CancellableTask` instead.
    pub(super) fn new(inner: InnerTask) -> Self {
        Self {
            inner,
            _t: PhantomData,
        }
    }
}

impl<T, InnerTask> Deref for IgnoreCancelCancellableTask<T, InnerTask>
where
    T: Send + Sync + Clone + 'static,
    InnerTask: CancellableTask<T>,
{
    type Target = InnerTask;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use crate::task::test_util::*;
    use crossbeam_channel::bounded;
    use proptest::prelude::*;

    #[test]
    fn test_notify() {
        let task = FreeCancellableTask::new(69).ignoring_cancellations();
        let (s, r) = bounded(1);

        task.notify_when_done(s);
        assert_result_eq!(r.recv().unwrap(), 69);
    }

    #[test]
    fn test_notify_cancel() {
        let task = FreeCancellableTask::new(69).ignoring_cancellations();
        let (s, r) = bounded(1);

        task.request_cancellation();
        task.notify_when_done(s);
        assert_result_eq!(r.recv().unwrap(), 69);
    }

    #[test]
    fn test_wait() {
        let task = FreeCancellableTask::new(69).ignoring_cancellations();
        assert_result_eq!(task.wait(), 69);
    }

    #[test]
    fn test_cancel() {
        let task = FreeCancellableTask::new(69).ignoring_cancellations();
        task.request_cancellation();
        assert_result_eq!(task.wait(), 69);
    }

    #[test]
    fn test_ct_invariants() {
        assert_cancellabletask_invariants(|| FreeCancellableTask::new(69).ignoring_cancellations());
    }

    proptest! {
        #[test]
        fn test_thread_safe(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| FreeCancellableTask::new(i).ignoring_cancellations());
        }
    }
}
