use crate::messaging::mailbox::Mailbox;
use crate::task::cancellable_task::CancellableTask;
use crate::threading::async_value::AsyncValue;
use crate::threading::single_use_cell::SingleUseCell;
use crossbeam_channel::Sender;
use std::sync::Arc;
use std::thread;

/// A CancellableTask that yields a value from the given function.
///
/// The function does not execute until `notify()` or `wait()` is called.
/// This struct is only usable in tests, because there's no way to interrupt the function once it starts, so if it infinitely loops, your program will never terminate without a SIGKILL.
pub struct FunctionCancellableTask<T, F>
where
    T: Send + Sync + Clone + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    async_msg: AsyncValue<Option<T>>,
    function_cell: SingleUseCell<F>,
}

impl<T, F> FunctionCancellableTask<T, F>
where
    T: Send + Sync + Clone + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    pub fn new(func: F) -> Self {
        let async_msg = AsyncValue::new();
        let function_cell = SingleUseCell::new(func);

        Self {
            async_msg,
            function_cell,
        }
    }
}

impl<T, F> CancellableTask<T> for FunctionCancellableTask<T, F>
where
    T: Send + Sync + Clone + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    fn notify_when_done(&self, notifiable: impl Mailbox<'static, Message = Option<T>> + 'static) {
        if let Some(f) = self.function_cell.take() {
            let async_msg_clone = self.async_msg.clone();
            thread::spawn(move || {
                async_msg_clone.send(Some(f()));
            });
        }
        self.async_msg.notify_when_done(notifiable);
    }

    fn request_cancellation(&self) -> () {
        self.async_msg.send(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::test_util::*;
    use proptest::prelude::*;

    #[test]
    fn returns_value() {
        let task = FunctionCancellableTask::new(|| 69);
        assert_result_eq!(task.wait(), 69);
    }

    #[test]
    fn returns_none_on_cancel() {
        let task = FunctionCancellableTask::new(|| 69);
        task.request_cancellation();
        assert_eq!(task.wait(), None);
    }

    #[test]
    fn test_ct_invariants() {
        assert_cancellabletask_invariants(|| FunctionCancellableTask::new(|| 69));
    }

    proptest! {
        #[test]
        fn test_thread_safe(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| FunctionCancellableTask::new(move || i));
        }
    }
}
