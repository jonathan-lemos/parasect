use crate::task::cancellable_task::CancellableTask;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Wraps a value in the CancellableTask trait.
///
/// cancel() will return None instead of the given value.
pub struct FreeCancellableTask<T: Send + Sync> {
    value: Arc<T>,
    cancelled: AtomicBool,
}

impl<T: Send + Sync> CancellableTask<T> for FreeCancellableTask<T> {
    fn request_cancellation(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn join(&self) -> Option<Arc<T>> {
        if self.cancelled.load(Ordering::Acquire) {
            None
        } else {
            Some(self.value.clone())
        }
    }
}

impl<T: Send + Sync> FreeCancellableTask<T> {
    /// Creates a CancellableTask out of a T.
    pub fn new<ArcLike: Into<Arc<T>>>(value: ArcLike) -> Self {
        Self {
            value: value.into(),
            cancelled: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::test_util::test_util::assert_result_eq;
    use crate::task::test_util::test_util::ResultLike;

    #[test]
    fn returns_value() {
        let task = FreeCancellableTask::<i64>::new(69);
        assert_result_eq!(task.join(), 69);
    }

    #[test]
    fn join_idempotent() {
        let task = FreeCancellableTask::<i64>::new(69);
        assert_result_eq!(task.join(), 69);
        assert_result_eq!(task.join(), 69);
    }

    #[test]
    fn returns_none_on_cancel() {
        let task = FreeCancellableTask::<i64>::new(69);
        task.request_cancellation();
        assert_eq!(task.join(), None);
    }
}
