use std::cell::Cell;
use std::sync::RwLock;
use crossbeam_channel::{bounded, Receiver, Sender};
use crate::task::cancellable_message::InnerValue::*;

enum InnerValue<T> {
    NotFinished,
    Finished(Option<T>)
}

/// Allows the sending of a single T that may be cancelled.
pub struct CancellableMessage<T> {
    sender: Sender<Option<T>>,
    receiver: Receiver<Option<T>>,
    inner_value: RwLock<Cell<InnerValue<T>>>
}

impl<T> CancellableMessage<T> {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(1);
        Self {
            sender,
            receiver,
            inner_value: RwLock::new(Cell::new(NotFinished))
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
    pub fn send(&self, value: T) -> () {
        // same logic as cancel() for why we can ignore the result
        let _ = self.sender.send(value);
    }

    /// Receives a reference to a value (Some) or a cancellation (None).
    /// Blocks until send() or cancel() is called.
    ///
    /// Repeated calls to this function will return the same value as the first call.
    pub fn recv(&self) -> Option<&T> {
        {
            let value_read = self.inner_value.read().unwrap().get_mut();
            if let Finished(v) = value_read {
                return v.map(|x| &x);
            }
        }
        {
            let value_write = self.inner_value.write().unwrap().get_mut();
            // we have to check again in case 2+ threads both try to set a value
            if let Finished(v) = value_write {
                return v.map(|x| &x);
            }
            let result = self.receiver.recv().unwrap();
            *value_write = Finished(result);
            if let Finished(v) = value_write {
                return v.map(|x| &x);
            }
            panic!("should never happen")
        }
    }

    /// Receives a value (Some) or a cancellation (None).
    /// Blocks until send() or cancel() is called.
    pub fn recv_into(&self) -> Option<T> {
        {
            let value_read = self.inner_value.read().unwrap().get_mut();
            if let Finished(v) = value_read {
                return v.map(|x| &x);
            }
        }
        self.receiver.recv().unwrap()
    }
}