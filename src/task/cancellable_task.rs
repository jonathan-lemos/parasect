use crate::task::ignore_cancel_cancellable_task::IgnoreCancelCancellableTask;
use crate::task::map_cancellable_task::MapValueCancellableTask;

/// An asynchronous task that can be cancelled.
///
/// Outputs a single value if uncancelled, or None if cancelled.
pub trait CancellableTask<T: Send + Sync>: Send + Sync {
    /// Ignores any .request_cancellation() calls on the CancellableTask.
    fn ignoring_cancellations(self) -> IgnoreCancelCancellableTask<T, Self>
    where
        Self: Sized,
    {
        IgnoreCancelCancellableTask::new(self)
    }

    /// Returns a reference to the inner value when it's generated.
    ///
    /// Blocks until the CancellableTask produces a value or is cancelled.
    /// None is returned if it's cancelled.
    fn join(&self) -> Option<&T>;

    /// Returns a clone of the inner value when it's generated.
    ///
    /// Blocks until the CancellableTask produces a value or is cancelled.
    /// None is returned if it's cancelled.
    fn join_clone(&self) -> Option<T>
    where
        T: Clone,
    {
        self.join().map(|x| x.clone())
    }

    /// Returns the inner value.
    ///
    /// Blocks until the CancellableTask produces a value or is cancelled.
    /// None is returned if it's cancelled.
    fn join_into(self) -> Option<T>;

    /// Maps the result of the CancellableTask.
    fn map<R: Send + Sync, Mapper>(
        self,
        mapper: Mapper,
    ) -> MapValueCancellableTask<T, R, Mapper, Self>
    where
        Self: Sized,
        Mapper: FnOnce(T) -> R + Send,
    {
        MapValueCancellableTask::new(self, mapper)
    }

    /// Request that the task stop as soon as possible.
    /// Returns before the cancellation has happened, but any join() or join_into() calls will return soon after.
    ///
    /// Calling this function does not guarantee that the result of join() will be None.
    fn request_cancellation(&self) -> ();
}
