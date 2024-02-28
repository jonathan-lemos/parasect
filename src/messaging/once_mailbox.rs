use crate::messaging::mailbox::Mailbox;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, RwLock};

/// A `Mailbox` that can only take a single value.
pub struct OnceMailbox<'a, T, M>
where
    T: Send + 'a,
    M: Mailbox<'a, Message = T> + 'a,
{
    _a: PhantomData<&'a ()>,
    _t: PhantomData<Mutex<T>>,
    send: Arc<RwLock<(M, bool)>>,
}

impl<'a, T> OnceMailbox<'a, T, Sender<T>>
where
    T: Send + 'a,
{
    pub fn new() -> (Self, Receiver<T>) {
        let (send, recv) = bounded(1);
        (
            Self {
                _t: PhantomData,
                _a: PhantomData,
                send: Arc::new(RwLock::new((send, false))),
            },
            recv,
        )
    }
}

impl<'a, T, M> OnceMailbox<'a, T, M>
where
    T: Send + 'a,
    M: Mailbox<'a, Message = T> + 'a,
{
    pub fn wrap(inner: M) -> Self {
        Self {
            _t: PhantomData,
            _a: PhantomData,
            send: Arc::new(RwLock::new((inner, false))),
        }
    }
}

impl<'a, T, M> Mailbox<'a> for OnceMailbox<'a, T, M>
where
    T: Send + Sync + 'a,
    M: Mailbox<'a, Message = T> + Sync,
{
    type Message = T;

    /// Sends a value, if a value was not already sent.
    ///
    /// Returns true if and only if the message was successfully sent.
    fn send_msg(&self, value: T) -> bool {
        {
            let read = self.send.read().unwrap();
            if read.1 {
                return false;
            }
        }

        let mut write = self.send.write().unwrap();
        write.0.send_msg(value);
        write.1 = true;
        return true;
    }
}

impl<'a, T, M> Clone for OnceMailbox<'a, T, M>
where
    T: Send + 'a,
    M: Mailbox<'a, Message = T>,
{
    fn clone(&self) -> Self {
        Self {
            _t: PhantomData,
            _a: PhantomData,
            send: self.send.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_once_sender() {
        let (send, recv) = OnceMailbox::new();

        assert_eq!(send.send_msg(69), true);
        assert_eq!(send.send_msg(70), false);

        assert_eq!(recv.try_recv(), Ok(69));
        assert!(recv.try_recv().is_err());
    }
}
