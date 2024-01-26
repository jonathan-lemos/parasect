use std::cell::{Cell, OnceCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use crate::task::cancellable_task::CancellableTask;

/// A CancellableTask that maps another CancellableTask using a function.
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
    /// Use the .map() method on a CancellableTask instead.
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

impl<TOld, TNew, Mapper, InnerTask> Deref for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
    where TOld: Send + Sync,
          TNew: Send + Sync,
          Mapper: FnOnce(Arc<TOld>) -> TNew,
          InnerTask: CancellableTask<TOld> {
    type Target = InnerTask;

    fn deref(&self) -> &Self::Target {
        &self.inner_task
    }
}

// Thread-safe because any mutations to the OnceCell require a write lock.
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

mod tests {
    use std::thread;
    use crate::task::cancellable_message::CancellableMessage;
    use super::*;
    use crate::task::cancellable_task::CancellableTask;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use crate::task::test_util::test_util::assert_result_eq;

    #[test]
    fn test_map_before() {
        let task = CancellableMessage::<i64>::new().map(|x| (*x) + 1);

        task.send(69);
        let val = task.join();

        assert_result_eq(val, 70);
    }

    #[test]
    fn test_map_after() {
        let task = CancellableMessage::<i64>::new().map(|x| (*x) + 1);

        let val = thread::scope(|scope| {
            let t = scope.spawn(|| task.join());

            scope.spawn(|| task.send(69));

            t.join().unwrap()
        });

        assert_result_eq(val, 70);
    }

    #[test]
    fn test_cancel_before() {
        let task = CancellableMessage::<i64>::new().map(|x| (*x) + 1);

        task.request_cancellation();
        let val = task.join();

        assert_eq!(val, None);
    }

    #[test]
    fn test_cancel_after() {
        let task = CancellableMessage::<i64>::new().map(|x| (*x) + 1);

        let val = thread::scope(|scope| {
            let t = scope.spawn(|| task.join());

            task.request_cancellation();

            t.join().unwrap()
        });

        assert_eq!(val, None);
    }
}
