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
    /// Wraps the given `Mailbox` in a `OnceMailbox` s.t. it can only take one message.
    ///
    /// If the `Mailbox` has been cloned, this function does not prevent the original `Mailbox` from receiving messages through its clones.
    #[allow(unused)]
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
    T: Send + 'a,
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
    use crossbeam_channel::unbounded;

    #[test]
    fn test_once_mailbox_new() {
        let (send, recv) = OnceMailbox::new();

        assert!(send.send_msg(69));
        assert!(!send.send_msg(70));

        assert_eq!(recv.try_recv(), Ok(69));
        assert!(recv.try_recv().is_err());
    }

    #[test]
    fn test_once_mailbox_wrap() {
        let (s, r) = unbounded();
        let mailbox = OnceMailbox::wrap(s);

        assert!(mailbox.send_msg(69));
        assert!(!mailbox.send_msg(70));

        assert_eq!(r.try_recv(), Ok(69));
        assert!(r.try_recv().is_err());
    }
}
