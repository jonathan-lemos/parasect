use crate::task::map_cancellable_task::{MapErrorCancellableTask, MapValueCancellableTask};

pub trait CancellableTask<T, E>: Send + Sync {
    // Request that the task stop as soon as possible.
    fn request_cancellation(&self) -> Result<(), E>;
    fn join(&self) -> Option<T>;

    fn map<R, Mapper>(self, mapper: Mapper) -> MapValueCancellableTask<T, R, E, Mapper, Self>
        where Self: Sized,
              Mapper: FnOnce(T) -> R {
        MapValueCancellableTask::new(self, mapper)
    }

    fn map_err<F, Mapper>(self, mapper: Mapper) -> MapErrorCancellableTask<T, E, F, Mapper, Self>
        where Self: Sized,
              Mapper: FnOnce(E) -> F {
        MapErrorCancellableTask::new(self, mapper)
    }
}
