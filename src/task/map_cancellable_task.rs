use std::marker::PhantomData;
use crate::task::cancellable_task::CancellableTask;

pub struct MapValueCancellableTask<TOld, TNew, E, Mapper, InnerTask>
    where Mapper: FnOnce(TOld) -> TNew,
          InnerTask: CancellableTask<TOld, E> {
    inner_task: InnerTask,
    mapper: Mapper,
    old: PhantomData<TOld>,
    new: PhantomData<TNew>,
    e: PhantomData<E>,
}

impl<TOld, TNew, E, Mapper, InnerTask> MapValueCancellableTask<TOld, TNew, E, Mapper, InnerTask>
    where Mapper: FnOnce(TOld) -> TNew,
          InnerTask: CancellableTask<TOld, E> {
    pub fn new(inner: InnerTask, mapper: Mapper) -> Self {
        Self {
            inner_task: inner,
            mapper,
            old: PhantomData,
            new: PhantomData,
            e: PhantomData,
        }
    }
}

impl<TOld, TNew, E, Mapper, InnerTask> CancellableTask<TNew, E> for MapValueCancellableTask<TOld, TNew, E, Mapper, InnerTask>
    where Mapper: FnOnce(TOld) -> TNew,
          InnerTask: CancellableTask<TOld, E> {
    fn request_cancellation(self) -> Result<(), E> {
        self.inner_task.request_cancellation()
    }

    fn join(self) -> TNew {
        (self.mapper)(self.inner_task.join())
    }
}

pub struct MapErrorCancellableTask<T, EOld, ENew, Mapper, InnerTask>
    where Mapper: FnOnce(EOld) -> ENew,
          InnerTask: CancellableTask<T, EOld> {
    inner_task: InnerTask,
    mapper: Mapper,
    old: PhantomData<EOld>,
    new: PhantomData<ENew>,
    t: PhantomData<T>,
}

impl<T, EOld, ENew, Mapper, InnerTask> CancellableTask<T, ENew> for MapErrorCancellableTask<T, EOld, ENew, Mapper, InnerTask>
    where Mapper: FnOnce(EOld) -> ENew,
          InnerTask: CancellableTask<T, EOld> {
    fn request_cancellation(self) -> Result<(), ENew> {
        self.inner_task.request_cancellation().map_err(self.mapper)
    }

    fn join(self) -> T {
        self.inner_task.join()
    }
}

impl<T, EOld, ENew, Mapper, InnerTask> MapErrorCancellableTask<T, EOld, ENew, Mapper, InnerTask>
    where Mapper: FnOnce(EOld) -> ENew,
          InnerTask: CancellableTask<T, EOld> {
    pub fn new(inner: InnerTask, mapper: Mapper) -> Self {
        Self {
            inner_task: inner,
            mapper,
            old: PhantomData,
            new: PhantomData,
            t: PhantomData,
        }
    }
}