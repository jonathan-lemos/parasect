use crate::messaging::mailbox::Mailbox;
use crate::task::cancellable_task::CancellableTask;
use crate::test_util::test_util::test_util::wait_for_condition;
use crate::threading::async_value::AsyncValue;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A CancellableTask only for testing.
///
/// Implements `Clone`, so you can `send()`/`notify()` from another thread even when the SUT needs to "own" the CancellableTask.
/// Will block indefinitely until `send()`/`notify()` is called.
#[derive(Clone)]
pub struct TestCancellableTask<T>
where
    T: Send + Sync + Clone + 'static,
{
    msg: AsyncValue<Option<T>>,
    sent_values: Arc<Mutex<Vec<T>>>,
    cancel_called_times: Arc<AtomicUsize>,
    notify_called_times: Arc<AtomicUsize>,
}

impl<T> TestCancellableTask<T>
where
    T: Send + Sync + Clone,
{
    pub fn new() -> Self {
        Self {
            msg: AsyncValue::new(),
            sent_values: Arc::new(Mutex::new(Vec::new())),
            notify_called_times: Arc::new(AtomicUsize::new(0)),
            cancel_called_times: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn send(&self, value: T) {
        self.sent_values.lock().unwrap().push(value.clone());
        self.msg.send(Some(value));
    }

    pub fn block_for_notify(&self, timeout: Duration, msg: impl ToString) {
        wait_for_condition(
            || self.notify_called_times.load(Ordering::Relaxed) > 0,
            timeout,
            msg,
        );
    }

    pub fn block_for_cancel(&self, timeout: Duration, msg: impl ToString) {
        wait_for_condition(
            || self.cancel_called_times.load(Ordering::Relaxed) > 0,
            timeout,
            msg,
        );
    }
}

impl<T> CancellableTask<T> for TestCancellableTask<T>
where
    T: Send + Sync + Clone,
{
    fn notify_when_done(&self, mailbox: impl Mailbox<'static, Message = Option<T>> + 'static) {
        self.msg.notify(mailbox);
        self.notify_called_times.fetch_add(1, Ordering::Relaxed);
    }

    fn request_cancellation(&self) -> () {
        self.msg.send_msg(None);
        self.cancel_called_times.fetch_add(1, Ordering::Relaxed);
    }
}
