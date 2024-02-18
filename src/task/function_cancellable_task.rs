use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_task::CancellableTask;
use crate::threading::single_use_cell::SingleUseCell;

/// A CancellableTask that yields a value from the given function.
///
/// The function does not execute until join() is called.
pub struct FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
    cancellable_message: CancellableMessage<T>,
    function: SingleUseCell<F>,
}

impl<T, F> FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
    pub fn new(func: F) -> Self {
        Self {
            cancellable_message: CancellableMessage::new(),
            function: SingleUseCell::new(func),
        }
    }
}

impl<T, F> CancellableTask<T> for FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
    fn join(&self) -> Option<&T> {
        if let Some(f) = self.function.take() {
            self.cancellable_message.send(f());
        }
        self.cancellable_message.recv()
    }

    fn join_into(self) -> Option<T> {
        self.join();
        self.cancellable_message.recv_into()
    }

    fn request_cancellation(&self) -> () {
        self.function.take();
        self.cancellable_message.cancel();
    }
}

unsafe impl<T, F> Send for FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
}

unsafe impl<T, F> Sync for FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::test_util::test_util::{assert_cancellabletask_invariants, assert_result_eq};
    use crate::task::test_util::test_util::{assert_cancellabletask_thread_safe, ResultLike};
    use proptest::prelude::*;

    #[test]
    fn returns_value() {
        let task = FunctionCancellableTask::new(|| 69);
        assert_result_eq!(task.join(), 69);
    }

    #[test]
    fn returns_none_on_cancel() {
        let task = FunctionCancellableTask::new(|| 69);
        task.request_cancellation();
        assert_eq!(task.join(), None);
    }

    #[test]
    fn test_ct_invariants() {
        assert_cancellabletask_invariants(|| FunctionCancellableTask::new(|| 69));
    }

    proptest! {
        #[test]
        fn test_thread_safe(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| FunctionCancellableTask::new(|| i));
        }
    }
}
