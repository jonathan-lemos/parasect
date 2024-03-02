use crate::messaging::mailbox::Mailbox;
use crate::task::ignore_cancel_cancellable_task::IgnoreCancelCancellableTask;
use crate::task::map_cancellable_task::MapValueCancellableTask;
use crossbeam_channel::bounded;

/// An asynchronous task that can be cancelled.
///
/// Outputs a single value if uncancelled, or None if cancelled.
pub trait CancellableTask<T>: Send + Sync
where
    T: Send + Sync + Clone + 'static,
{
    /// Ignores any .request_cancellation() calls on the CancellableTask.
    #[allow(unused)]
    fn ignoring_cancellations(self) -> IgnoreCancelCancellableTask<T, Self>
    where
        Self: Sized,
    {
        IgnoreCancelCancellableTask::new(self)
    }

    /// Sends a message to the `Mailbox` when the task completes.
    ///
    /// If it's already done, send the message immediately.
    fn notify_when_done(&self, mailbox: impl Mailbox<'static, Message = Option<T>> + 'static);

    /// Maps the result of the CancellableTask.
    fn map<R>(
        self,
        mapper: impl FnOnce(T) -> R + Send + 'static,
    ) -> MapValueCancellableTask<T, R, Self>
    where
        Self: Sized,
        R: Send + Sync + Clone + 'static,
    {
        MapValueCancellableTask::new(self, mapper)
    }

    /// Request that the task stop as soon as possible.
    /// Returns before the cancellation has happened, but any wait() calls and notify() subscribers will complete soon after.
    ///
    /// Calling this function does not guarantee that the result of notify()/wait() will be None.
    fn request_cancellation(&self) -> ();

    /// Returns a clone of the task result when it's generated.
    ///
    /// Blocks until the CancellableTask produces a value or is cancelled.
    /// None is returned if it's cancelled.
    fn wait(&self) -> Option<T> {
        let (send, recv) = bounded(1);
        self.notify_when_done(send);
        recv.recv().unwrap()
    }
}
