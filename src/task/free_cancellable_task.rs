use std::sync::atomic::{AtomicBool, Ordering};
use crate::task::cancellable_task::CancellableTask;

pub struct FreeCancellableTask<T: Send + Sync> {
    value: T,
    cancelled: AtomicBool
}

impl<T: Send + Sync> CancellableTask<T> for FreeCancellableTask<T> {
    fn request_cancellation(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn join(&self) -> Option<&T> {
        if self.cancelled.load(Ordering::Acquire) {
            Some(&self.value)
        }
        else {
            None
        }
    }

    fn join_into(self) -> Option<T> {
        if self.cancelled.load(Ordering::Acquire) {
            Some(self.value)
        }
        else {
            None
        }
    }
}

impl<T: Send + Sync> FreeCancellableTask<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}
