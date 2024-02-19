use crate::task::cancellable_task::CancellableTask;
use std::marker::PhantomData;
use std::ops::Deref;

/// Wraps a CancellableTask, but silently drops all cancellations.
///
/// Do not instantiate directly. Call .ignore_cancellations() instead.
pub struct IgnoreCancelCancellableTask<T, InnerTask>
where
    T: Send + Sync,
    InnerTask: CancellableTask<T>,
{
    inner: InnerTask,
    _t: PhantomData<T>,
}

impl<T, InnerTask> CancellableTask<T> for IgnoreCancelCancellableTask<T, InnerTask>
where
    T: Send + Sync,
    InnerTask: CancellableTask<T>,
{
    fn join(&self) -> Option<&T> {
        self.inner.join()
    }

    fn join_into(self) -> Option<T> {
        self.inner.join_into()
    }

    fn request_cancellation(&self) -> () {}
}

impl<T, InnerTask> IgnoreCancelCancellableTask<T, InnerTask>
where
    T: Send + Sync,
    InnerTask: CancellableTask<T>,
{
    /// Do not instantiate directly. Use .ignore_cancellations() on any CancellableTask instead.
    pub fn new(inner: InnerTask) -> Self {
        Self {
            inner,
            _t: PhantomData,
        }
    }
}

impl<T, InnerTask> Deref for IgnoreCancelCancellableTask<T, InnerTask>
where
    T: Send + Sync,
    InnerTask: CancellableTask<T>,
{
    type Target = InnerTask;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use crate::task::cancellable_task::CancellableTask;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use crate::task::test_util::*;
    use proptest::prelude::*;

    #[test]
    fn test_join() {
        let task = FreeCancellableTask::new(69).ignoring_cancellations();
        assert_result_eq!(task.join(), 69);
    }

    #[test]
    fn test_cancel() {
        let task = FreeCancellableTask::new(69).ignoring_cancellations();
        task.request_cancellation();
        assert_result_eq!(task.join(), 69);
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
