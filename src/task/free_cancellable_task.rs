use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::task::cancellable_task::CancellableTask;

pub struct FreeCancellableTask<T: Send + Sync> {
    value: Arc<T>,
    cancelled: AtomicBool
}

impl<T: Send + Sync> CancellableTask<T> for FreeCancellableTask<T> {
    fn request_cancellation(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn join(&self) -> Option<Arc<T>> {
        if self.cancelled.load(Ordering::Acquire) {
            Some(self.value.clone())
        }
        else {
            None
        }
    }
}

impl<T: Send + Sync> FreeCancellableTask<T> {
    pub fn new<ArcLike: Into<Arc<T>>>(value: ArcLike) -> Self {
        Self { value: value.into(), cancelled: AtomicBool::new(false) }
    }
}
