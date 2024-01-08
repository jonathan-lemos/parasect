use crate::task::map_cancellable_task::MapValueCancellableTask;

// Represents an asynchronous task that can be cancelled.
pub trait CancellableTask<T>: Send + Sync
where T: Send + Sync {
    // Request that the task stop as soon as possible.
    // Returns before the cancellation has happened, but any join() or join_into() calls will return soon after.
    fn request_cancellation(&self) -> ();

    // Returns a reference to the inner value when it's generated.
    //
    // Blocks until the CancellableTask produces a value or is cancelled.
    // None is returned if it's cancelled.
    fn join(&self) -> Option<&T>;

    // Returns the inner value, consuming the CancellableTask.
    //
    // Blocks until the CancellableTask produces a value or is cancelled.
    // None is returned if it's cancelled.
    fn join_into(self) -> Option<T>;

    fn map<R, Mapper>(self, mapper: Mapper) -> MapValueCancellableTask<T, R, Mapper, Self>
        where Self: Sized,
              Mapper: FnOnce(T) -> R {
        MapValueCancellableTask::new(self, mapper)
    }

}
