use crate::messaging::mailbox::Mailbox;
use crate::task::cancellable_task::CancellableTask;
use crate::threading::async_value::AsyncValue;
use crate::threading::once_actor::OnceActor;

/// A CancellableTask that maps another CancellableTask using a function.
pub struct MapValueCancellableTask<TOld, TNew, InnerTask>
where
    TOld: Send + Sync + Clone + 'static,
    TNew: Send + Sync + Clone + 'static,
    InnerTask: CancellableTask<TOld>,
{
    inner_task: InnerTask,
    inner_task_reactor: OnceActor<'static, Option<TOld>>,
    mapped_value: AsyncValue<Option<TNew>>,
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
        let mapped_value = AsyncValue::new();
        let mapped_value_clone = mapped_value.clone();
        let inner_task_reactor = OnceActor::spawn(move |told: Option<TOld>| {
            mapped_value_clone.send(told.map(mapper));
        });

        inner.notify_when_done(inner_task_reactor.mailbox());

        Self {
            inner_task: inner,
            inner_task_reactor,
            mapped_value,
        }
    }
}

impl<TOld, TNew, InnerTask> CancellableTask<TNew> for MapValueCancellableTask<TOld, TNew, InnerTask>
where
    TOld: Send + Sync + Clone + 'static,
    TNew: Send + Sync + Clone + 'static,
    InnerTask: CancellableTask<TOld>,
{
    fn notify_when_done(&self, mailbox: impl Mailbox<'static, Message = Option<TNew>> + 'static) {
        self.mapped_value.notify_when_done(mailbox);
    }

    fn request_cancellation(&self) -> () {
        self.mapped_value.send(None);
        self.inner_task.request_cancellation();
        self.inner_task_reactor.stop();
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
    fn test_map_before_wait() {
        let test_task = TestCancellableTask::new();
        let task = test_task.clone().map(|x| x + 1);

        test_task.send(69);
        let val = task.wait();

        assert_result_eq!(val, 70);
    }

    #[test]
    fn test_map_after_wait() {
        let test_task = TestCancellableTask::new();
        let task = test_task.clone().map(|x| x + 1);

        let val = thread::scope(|scope| {
            let handle = scope.spawn(|| task.wait());
            test_task.send(69);
            handle.join().unwrap()
        });

        assert_result_eq!(val, 70);
    }

    #[test]
    fn test_cancel_before_wait() {
        let task = FunctionCancellableTask::new(|| 69).map(|x| x + 1);

        task.request_cancellation();
        let val = task.wait();

        assert_eq!(val, None);
    }

    #[test]
    fn test_cancel_after_wait() {
        let task = FunctionCancellableTask::new(|| 69).map(|x| x + 1);

        let val = thread::scope(|scope| {
            let t = scope.spawn(|| task.wait());

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
