use crate::task::cancellable_task::CancellableTask;
use crate::threading::async_value::AsyncValue;
use crate::threading::once_reactor::OnceReactor;
use crate::threading::single_use_cell::SingleUseCell;
use crossbeam_channel::Sender;
use std::marker::PhantomData;
use std::sync::Arc;

/// A CancellableTask that maps another CancellableTask using a function.
pub struct MapValueCancellableTask<TOld, TNew, InnerTask>
where
    TOld: Send + Sync + Clone + 'static,
    TNew: Send + Sync + Clone + 'static,
    InnerTask: CancellableTask<TOld>,
{
    inner_task: SingleUseCell<InnerTask>,
    inner_task_reactor: OnceReactor<TOld>,
    mapped_value: Arc<AsyncValue<TNew>>,
}

impl<TOld, TNew, InnerTask> MapValueCancellableTask<TOld, TNew, InnerTask>
where
    TOld: Send + Sync + Clone + 'static,
    TNew: Send + Sync + Clone + 'static,
    InnerTask: CancellableTask<TOld>,
{
    /// Use the .map() method on a CancellableTask instead.
    pub(super) fn new(
        inner: InnerTask,
        mapper: impl FnOnce(TOld) -> TNew + Send + 'static,
    ) -> Self {
        let mapped_value = Arc::new(AsyncValue::new());
        let mapped_value_clone = mapped_value.clone();
        let inner_task_reactor = OnceReactor::new(move |told| {
            mapped_value_clone.send(mapper(told));
        });

        inner.notify_when_done()

        Self {
            inner_task: SingleUseCell::new(inner),
            inner_task_reactor,
            mapped_value: Arc::new(AsyncValue::new()),
        }
    }
}

impl<TOld, TNew, Mapper, InnerTask> CancellableTask<TNew>
    for MapValueCancellableTask<TOld, TNew, Mapper, InnerTask>
where
    TOld: Send + Sync + Clone + 'static,
    TNew: Send + Sync + Clone + 'static,
    Mapper: FnOnce(TOld) -> TNew + Send,
    InnerTask: CancellableTask<TOld>,
{
    fn notify_when_done(&self, sender: Sender<Option<TNew>>) {

    }

    fn request_cancellation(&self) -> () {
        todo!();
        /*
        let _ = self.inner_value.set(None);
        self.mapper.take();
        self.inner_task.take().inspect(|t| t.request_cancellation());
         */
    }
}

#[cfg(test)]
mod tests {
    use crate::task::cancellable_task::CancellableTask;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use crate::task::function_cancellable_task::FunctionCancellableTask;
    use crate::task::test_cancellable_task::TestCancellableTask;
    use crate::task::test_util::*;
    use proptest::proptest;
    use std::thread;

    #[test]
    fn test_map_before() {
        let test_task = TestCancellableTask::new();
        let task = test_task.clone().map(|x| x + 1);

        test_task.send(69);
        let val = task.join();

        assert_result_eq!(val, 70);
    }

    #[test]
    fn test_map_after() {
        let test_task = TestCancellableTask::new();
        let task = test_task.clone().map(|x| x + 1);

        let val = thread::scope(|scope| {
            let handle = scope.spawn(|| task.join());
            test_task.send(69);
            handle.join().unwrap()
        });

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
        assert_higher_order_cancellabletask_invariants(69, 70, || {
            let tc = TestCancellableTask::new();
            (tc.clone(), tc.map(|x| x + 1))
        });
    }

    proptest! {
        #[test]
        fn test_thread_safe(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| FreeCancellableTask::new(i).map(|x| x + 1));
        }
    }
}
