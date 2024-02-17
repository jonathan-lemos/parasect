use crate::task::cancellable_task::CancellableTask;
use crate::task::map_cancellable_task::ValueState::*;
use std::cell::{Cell, UnsafeCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

enum ValueState<T: Send> {
    Unset,
    Cancelled,
    Set(T),
}

/// A CancellableTask that maps another CancellableTask using a function.
pub struct MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(&TOld) -> TNew,
    InnerTask: CancellableTask<TOld>,
{
    told_phantom: PhantomData<TOld>,
    mapper: Cell<Option<Mapper>>,
    inner_task: InnerTask,
    value_cell: RwLock<UnsafeCell<ValueState<TNew>>>,
}

impl<TOld, TNew, Mapper, InnerTask> MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(&TOld) -> TNew,
    InnerTask: CancellableTask<TOld>,
{
    /// Use the .map() method on a CancellableTask instead.
    pub fn new(inner: InnerTask, mapper: Mapper) -> Self {
        Self {
            told_phantom: PhantomData,
            mapper: Cell::new(Some(mapper)),
            inner_task: inner,
            value_cell: RwLock::new(UnsafeCell::new(Unset)),
        }
    }
}

impl<TOld, TNew, Mapper, InnerTask> CancellableTask<TNew>
    for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(&TOld) -> TNew,
    InnerTask: CancellableTask<TOld>,
{
    fn request_cancellation(&self) -> () {
        self.inner_task.request_cancellation()
    }

    fn join(&self) -> Option<&TNew> {
        {
            let read_lock = self.value_cell.read().unwrap();

            // unsafe needed to get a reference not tied to the lifetime of the guard
            let immut_ref = unsafe { read_lock.get().as_ref().unwrap() };

            match immut_ref {
                Set(v) => return Some(v),
                Cancelled => return None,
                Unset => {}
            }
        }
        {
            let mut write_lock = self.value_cell.write().unwrap();

            if let Some(mapper) = self.mapper.take() {
                let mut_ref = write_lock.get_mut();
                *mut_ref = match self.inner_task.join() {
                    Some(v) => Set(mapper(v)),
                    None => Cancelled,
                };
            }

            // unsafe needed to get a reference not tied to the lifetime of the guard
            match unsafe { write_lock.get().as_ref().unwrap() } {
                Set(v) => Some(v),
                Cancelled => None,
                Unset => panic!("should never happen"),
            }
        }
    }
}

impl<TOld, TNew, Mapper, InnerTask> Deref for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(&TOld) -> TNew,
    InnerTask: CancellableTask<TOld>,
{
    type Target = InnerTask;

    fn deref(&self) -> &Self::Target {
        &self.inner_task
    }
}

// Thread-safe because any mutations to the OnceCell require a write lock.
unsafe impl<TOld, TNew, Mapper, InnerTask> Send
    for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(&TOld) -> TNew,
    InnerTask: CancellableTask<TOld>,
{
}

unsafe impl<TOld, TNew, Mapper, InnerTask> Sync
    for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(&TOld) -> TNew,
    InnerTask: CancellableTask<TOld>,
{
}

#[cfg(test)]
mod tests {
    use crate::task::cancellable_message::CancellableMessage;
    use crate::task::cancellable_task::CancellableTask;
    use crate::task::test_util::test_util::assert_result_eq;
    use crate::task::test_util::test_util::ResultLike;
    use std::thread;

    #[test]
    fn test_map_before() {
        let task = CancellableMessage::<i64>::new().map(|x| (*x) + 1);

        task.send(69);
        let val = task.join();

        assert_result_eq!(val, 70);
    }

    #[test]
    fn test_join_idempotent() {
        let task = CancellableMessage::<i64>::new().map(|x| (*x) + 1);

        task.send(69);
        let val = task.join();
        let val2 = task.join();

        assert_eq!(val, val2);
        assert_result_eq!(val, 70);
    }

    #[test]
    fn test_map_after() {
        let task = CancellableMessage::<i64>::new().map(|x| (*x) + 1);

        let val = thread::scope(|scope| {
            let t = scope.spawn(|| task.join());

            scope.spawn(|| task.send(69));

            t.join().unwrap()
        });

        assert_result_eq!(val, 70);
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
