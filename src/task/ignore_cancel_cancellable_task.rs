use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use crate::task::cancellable_task::CancellableTask;

/// Wraps a CancellableTask, but silently drops all cancellations.
///
/// Do not instantiate directly. Call .ignore_cancellations() instead.
pub struct IgnoreCancelCancellableTask<T, InnerTask>
    where T: Send + Sync,
          InnerTask: CancellableTask<T> {
    inner: InnerTask,
    _t: PhantomData<T>,
}

impl<T, InnerTask> CancellableTask<T> for IgnoreCancelCancellableTask<T, InnerTask>
    where T: Send + Sync,
          InnerTask: CancellableTask<T> {
    fn request_cancellation(&self) -> () {}

    fn join(&self) -> Option<Arc<T>> {
        self.inner.join()
    }
}

impl<T, InnerTask> IgnoreCancelCancellableTask<T, InnerTask>
    where T: Send + Sync,
          InnerTask: CancellableTask<T> {
    /// Do not instantiate directly. Use .ignore_cancellations() on any CancellableTask instead.
    pub fn new(inner: InnerTask) -> Self {
        Self {
            inner,
            _t: PhantomData
        }
    }
}

impl<T, InnerTask> Deref for IgnoreCancelCancellableTask<T, InnerTask>
    where T: Send + Sync,
          InnerTask: CancellableTask<T> {
    type Target = InnerTask;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use crate::task::cancellable_task::CancellableTask;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use crate::task::test_util::test_util::assert_result_eq;

    #[test]
    fn test_join() {
        let task = FreeCancellableTask::new(69).ignore_cancellations();
        assert_result_eq(task.join(), 69);
    }

    #[test]
    fn test_cancel() {
        let task = FreeCancellableTask::new(69).ignore_cancellations();
        task.request_cancellation();
        assert_result_eq(task.join(), 69);
    }
}