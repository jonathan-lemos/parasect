use std::cell::{Cell, OnceCell};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use crate::task::cancellable_task::CancellableTask;

pub struct MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where TOld: Send + Sync,
          TNew: Send + Sync,
          Mapper: FnOnce(Arc<TOld>) -> TNew,
          InnerTask: CancellableTask<TOld> {
    told_phantom: PhantomData<TOld>,
    mapper: Cell<Option<Mapper>>,
    inner_task: InnerTask,
    value_cell: RwLock<OnceCell<Option<Arc<TNew>>>>,
}

impl<TOld, TNew, Mapper, InnerTask> MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where TOld: Send + Sync,
          TNew: Send + Sync,
          Mapper: FnOnce(Arc<TOld>) -> TNew,
          InnerTask: CancellableTask<TOld> {
    pub fn new(inner: InnerTask, mapper: Mapper) -> Self {
        Self {
            told_phantom: PhantomData,
            mapper: Cell::new(Some(mapper)),
            inner_task: inner,
            value_cell: RwLock::new(OnceCell::new()),
        }
    }
}

impl<TOld, TNew, Mapper, InnerTask> CancellableTask<TNew> for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where TOld: Send + Sync,
          TNew: Send + Sync,
          Mapper: FnOnce(Arc<TOld>) -> TNew,
          InnerTask: CancellableTask<TOld> {
    fn request_cancellation(&self) -> () {
        self.inner_task.request_cancellation()
    }

    fn join(&self) -> Option<Arc<TNew>> {
        {
            let read_lock = self.value_cell.read().unwrap();
            if let Some(v) = read_lock.get() {
                return v.clone();
            }
        }
        {
            let write_lock = self.value_cell.write().unwrap();

            if let Some(mapper) = self.mapper.take() {
                let _ = write_lock.set(self.inner_task.join().map(mapper).map(Arc::new));
            }

            write_lock.get().unwrap().clone()
        }
    }
}

unsafe impl<TOld, TNew, Mapper, InnerTask> Send for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where TOld: Send + Sync,
          TNew: Send + Sync,
          Mapper: FnOnce(Arc<TOld>) -> TNew,
          InnerTask: CancellableTask<TOld> {}

unsafe impl<TOld, TNew, Mapper, InnerTask> Sync for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where TOld: Send + Sync,
          TNew: Send + Sync,
          Mapper: FnOnce(Arc<TOld>) -> TNew,
          InnerTask: CancellableTask<TOld> {}
