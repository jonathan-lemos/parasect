use crate::threading::mailbox::Mailbox;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::{Arc, RwLock};

/// A `Mailbox` that can only take a single value.
pub struct OnceMailbox<'a, T>
where
    T: Send + 'a,
{
    send: Arc<RwLock<(Box<dyn Mailbox<'a, Message = T> + 'a>, bool)>>,
}

impl<'a, T> OnceMailbox<'a, T>
where
    T: Send + 'a,
{
    pub fn new() -> (Self, Receiver<T>) {
        let (send, recv) = bounded(1);
        (
            Self {
                send: Arc::new(RwLock::new((Box::new(send), false))),
            },
            recv,
        )
    }

    pub fn wrap(inner: impl Mailbox<'a, Message = T> + 'a) -> Self {
        Self {
            send: Arc::new(RwLock::new((Box::new(inner), false))),
        }
    }
}

impl<'a, T> Mailbox<'a> for OnceMailbox<'a, T>
where
    T: Send + 'a,
{
    type Message = T;

    /// Sends a value, if a value was not already sent.
    ///
    /// Returns true if and only if the message was successfully sent.
    fn give_message(&self, value: T) -> bool {
        {
            let read = self.send.read().unwrap();
            if read.1 {
                return false;
            }
        }

        let mut write = self.send.write().unwrap();
        write.0.give_message(value);
        write.1 = true;
        return true;
    }
}

impl<'a, T> Clone for OnceMailbox<'a, T>
where
    T: Send + 'a,
{
    fn clone(&self) -> Self {
        Self {
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

        assert_eq!(send.give_message(69), true);
        assert_eq!(send.give_message(70), false);

        assert_eq!(recv.try_recv(), Ok(69));
        assert!(recv.try_recv().is_err());
    }
}
