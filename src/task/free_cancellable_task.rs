use crate::task::cancellable_task::CancellableTask;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Wraps a value in the CancellableTask trait.
///
/// cancel() will return None instead of the given value.
pub struct FreeCancellableTask<T: Send + Sync> {
    value: T,
    cancelled: AtomicBool,
    value_was_returned: AtomicBool,
}

impl<T: Send + Sync> CancellableTask<T> for FreeCancellableTask<T> {
    fn join(&self) -> Option<&T> {
        if self.cancelled.load(Ordering::Relaxed)
            && !self.value_was_returned.load(Ordering::Relaxed)
        {
            None
        } else {
            self.value_was_returned.store(true, Ordering::Relaxed);
            Some(&self.value)
        }
    }

    fn join_into(self) -> Option<T> {
        Some(self.value)
    }

    fn request_cancellation(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl<T: Send + Sync> FreeCancellableTask<T> {
    /// Creates a CancellableTask out of a T.
    pub fn new(value: T) -> Self {
        Self {
            value,
            cancelled: AtomicBool::new(false),
            value_was_returned: AtomicBool::new(false),
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
