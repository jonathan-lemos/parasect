use std::cell::OnceCell;
use std::sync::{Arc, RwLock};
use crossbeam_channel::{bounded, Receiver, Sender};


/// Allows the sending of a single T that may be cancelled.
pub struct CancellableMessage<T: Send + Sync> {
    sender: Sender<Option<Arc<T>>>,
    receiver: Receiver<Option<Arc<T>>>,
    inner_value: Arc<RwLock<OnceCell<Option<Arc<T>>>>>,
}

impl<T: Send + Sync> CancellableMessage<T> {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(1);
        Self {
            sender,
            receiver,
            inner_value: Arc::new(RwLock::new(OnceCell::new()))
        }
    }

    /// Cancel the message.
    /// All future cancel() or send() calls will be ignored.
    pub fn cancel(&self) -> () {
        // if this fails to send, it's because
        // 1) there is already a message in the channel, or
        // 2) the channel is disconnected
        //
        // in both cases, we don't care if this message is dropped,
        // so it's unnecessary to handle it
        let _ = self.sender.send(None);
    }

    /// Send a value.
    /// All future cancel() or send() calls will be ignored.
    pub fn send<ArcLike: Into<Arc<T>>>(&self, value: ArcLike) -> () {
        // same logic as cancel() for why we can ignore the result
        let _ = self.sender.send(Some(value.into()));
    }

    /// Receives a reference to a value (Some) or a cancellation (None).
    /// Blocks until send() or cancel() is called.
    ///
    /// Repeated calls to this function will return the same value as the first call.
    pub fn recv(&self) -> Option<Arc<T>> {
        {
            let value_read = self.inner_value.read().unwrap();

            if let Some(v) = value_read.get() {
                return v.clone();
            }
        }
        {
            let value_write = self.inner_value.write().unwrap();
            if let Err(existing) = value_write.set(self.receiver.recv().unwrap()) {
                return existing.clone();
            }

            if let Some(v) = value_write.get() {
                return v.clone();
            }

            panic!("should never happen")
        }

    }
}

impl<T: Send + Sync> Clone for CancellableMessage<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            inner_value: self.inner_value.clone()
        }
    }
}

unsafe impl<T> Send for CancellableMessage<T> where T: Send + Sync {}
unsafe impl<T> Sync for CancellableMessage<T> where T: Send + Sync {}

#[cfg(test)]
mod tests {
    use std::thread;
    use super::*;

    #[test]
    fn test_send_recv() {
        let cm = CancellableMessage::<i64>::new();

        let answer = thread::scope(|scope| {
            let result = scope.spawn(|| cm.recv());
            scope.spawn(|| cm.send(69));

            result.join().unwrap()
        }).map(|x| (*x).clone());

        assert_eq!(answer, Some(69));
    }

    #[test]
    fn test_cancel() {
        let cm = CancellableMessage::<i64>::new();

        let answer = thread::scope(|scope| {
            let result = scope.spawn(|| cm.recv());
            scope.spawn(|| cm.cancel());

            result.join().unwrap()
        });

        assert_eq!(answer, None);
    }
}
