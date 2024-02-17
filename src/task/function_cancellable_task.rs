use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_task::CancellableTask;
use std::cell::UnsafeCell;
use std::sync::{Arc, RwLock};

/// A CancellableTask that yields a value from the given function.
///
/// The function does not execute until join() is called.
pub struct FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
    cancellable_message: CancellableMessage<T>,
    function: Arc<RwLock<UnsafeCell<Option<F>>>>,
}

impl<T, F> FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
    pub fn new(func: F) -> Self {
        Self {
            cancellable_message: CancellableMessage::new(),
            function: Arc::new(RwLock::new(UnsafeCell::new(Some(func)))),
        }
    }
}

impl<T, F> CancellableTask<T> for FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
    fn request_cancellation(&self) -> () {
        {
            let read = self.function.read().unwrap();

            // safe because no read blocks mutate the pointed-to data
            let immut_ref = unsafe { read.get().as_ref().unwrap() };
            if immut_ref.is_none() {
                return;
            }
        }
        {
            let mut write = self.function.write().unwrap();

            *write.get_mut() = None;
            self.cancellable_message.cancel();
        }
    }

    fn join(&self) -> Option<&T> {
        {
            let read = self.function.read().unwrap();

            // safe because no read blocks mutate the pointed-to data
            let immut_ref = unsafe { read.get().as_ref().unwrap() };
            if immut_ref.is_none() {
                return self.cancellable_message.join();
            }
        }
        {
            let mut write = self.function.write().unwrap();

            let f = match write.get_mut().take() {
                None => return self.cancellable_message.join(),
                Some(f) => f,
            };
            self.cancellable_message.send(f());
            self.cancellable_message.join()
        }
    }
}

impl<T, F> Clone for FunctionCancellableTask<T, F>
where
    T: Send + Sync,
    F: FnOnce() -> T,
{
    fn clone(&self) -> Self {
        Self {
            cancellable_message: self.cancellable_message.clone(),
            function: self.function.clone(),
        }
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
    use crate::task::test_util::test_util::assert_result_eq;
    use crate::task::test_util::test_util::ResultLike;

    #[test]
    fn returns_value() {
        let task = FunctionCancellableTask::new(|| 69);
        assert_result_eq!(task.join(), 69);
    }

    #[test]
    fn join_idempotent() {
        let task = FunctionCancellableTask::new(|| 69);
        assert_result_eq!(task.join(), 69);
        assert_result_eq!(task.join(), 69);
    }

    #[test]
    fn returns_none_on_cancel() {
        let task = FunctionCancellableTask::new(|| 69);
        task.request_cancellation();
        assert_eq!(task.join(), None);
    }
}
