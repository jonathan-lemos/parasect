use crate::task::cancellable_task::CancellableTask;
use crate::threading::single_use_cell::SingleUseCell;
use std::marker::PhantomData;
use std::sync::OnceLock;

/// A CancellableTask that maps another CancellableTask using a function.
pub struct MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(TOld) -> TNew + Send,
    InnerTask: CancellableTask<TOld>,
{
    told_phantom: PhantomData<TOld>,
    mapper: SingleUseCell<Mapper>,
    inner_task: SingleUseCell<InnerTask>,
    inner_value: OnceLock<Option<TNew>>,
}

impl<TOld, TNew, Mapper, InnerTask> MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(TOld) -> TNew + Send,
    InnerTask: CancellableTask<TOld>,
{
    /// Use the .map() method on a CancellableTask instead.
    pub fn new(inner: InnerTask, mapper: Mapper) -> Self {
        Self {
            told_phantom: PhantomData,
            mapper: SingleUseCell::new(mapper),
            inner_task: SingleUseCell::new(inner),
            inner_value: OnceLock::new(),
        }
    }
}

impl<TOld, TNew, Mapper, InnerTask> CancellableTask<TNew>
    for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync,
    TNew: Send + Sync,
    Mapper: FnOnce(TOld) -> TNew + Send,
    InnerTask: CancellableTask<TOld>,
{
    fn join(&self) -> Option<&TNew> {
        self.inner_value
            .get_or_init(|| {
                self.inner_task
                    .take()
                    .and_then(|t| t.join_into().map(self.mapper.take().unwrap()))
            })
            .as_ref()
    }

    fn join_into(mut self) -> Option<TNew> {
        self.join();
        self.inner_value.take().unwrap()
    }

    fn request_cancellation(&self) -> () {
        let _ = self.inner_value.set(None);
        self.mapper.take();
        self.inner_task.take().inspect(|t| t.request_cancellation());
    }
}

#[cfg(test)]
mod tests {
    use crate::task::cancellable_task::CancellableTask;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use crate::task::function_cancellable_task::FunctionCancellableTask;
    use crate::task::test_util::*;
    use proptest::proptest;
    use std::thread;

    #[test]
    fn test_map_before() {
        let task = FreeCancellableTask::new(69).map(|x| x + 1);

        let val = task.join();

        assert_result_eq!(val, 70);
    }

    #[test]
    fn test_map_after() {
        let task = FunctionCancellableTask::new(|| 69).map(|x| x + 1);

        let val = task.join();

        assert_result_eq!(val, 70);
    }

    #[test]
    fn test_cancel_before() {
        let task = FunctionCancellableTask::new(|| 69).map(|x| x + 1);

        task.request_cancellation();
        let val = task.join();

        assert_eq!(val, None);
    }

    #[test]
    fn test_cancel_after() {
        let task = FunctionCancellableTask::new(|| 69).map(|x| x + 1);

        let val = thread::scope(|scope| {
            let t = scope.spawn(|| task.join());

            task.request_cancellation();

            t.join().unwrap()
        });

        assert_eq!(val, None);
    }

    #[test]
    fn test_ct_invariants() {
        assert_cancellabletask_invariants(|| FreeCancellableTask::new(68).map(|x| x + 1));
    }

    proptest! {
        #[test]
        fn test_thread_safe(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| FreeCancellableTask::new(i).map(|x| x + 1));
        }
    }
}
