use crate::task::cancellable_message::CancellableMessage;
use crate::task::cancellable_task::CancellableTask;
use crate::test_util::test_util::test_util::wait_for_condition;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A CancellableTask only for testing.
///
/// Implements Clone, so you can `send()`/`join()` from another thread even when the SUT needs to "own" the CancellableTask.
/// Will block indefinitely until `send()` is called.
#[derive(Debug, Clone)]
pub struct TestCancellableTask<T>
where
    T: Send + Sync + Clone,
{
    msg: Arc<CancellableMessage<T>>,
    sent_values: Arc<Mutex<Vec<T>>>,
    cancel_called_times: Arc<AtomicUsize>,
    join_called_times: Arc<AtomicUsize>,
}

impl<T> TestCancellableTask<T>
where
    T: Send + Sync + Clone,
{
    pub fn new() -> Self {
        Self {
            msg: Arc::new(CancellableMessage::new()),
            sent_values: Arc::new(Mutex::new(Vec::new())),
            cancel_called_times: Arc::new(AtomicUsize::new(0)),
            join_called_times: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn send(&self, value: T) {
        self.sent_values.lock().unwrap().push(value.clone());
        self.msg.send(value);
    }

    pub fn wait_for_join(&self) {
        wait_for_condition(|| self.join_called_times.load(Ordering::Relaxed) > 0);
    }

    pub fn wait_for_cancel(&self) {
        wait_for_condition(|| self.cancel_called_times.load(Ordering::Relaxed) > 0);
    }
}

impl<T> CancellableTask<T> for TestCancellableTask<T>
where
    T: Send + Sync + Clone,
{
    fn join(&self) -> Option<&T> {
        let r = self.msg.join();
        self.join_called_times.fetch_add(1, Ordering::Relaxed);
        r
    }

    fn join_into(self) -> Option<T> {
        let r = self.msg.join_clone();
        self.join_called_times.fetch_add(1, Ordering::Relaxed);
        r
    }

    fn request_cancellation(&self) -> () {
        self.cancel_called_times.fetch_add(1, Ordering::Relaxed);
        self.msg.request_cancellation();
    }
}
