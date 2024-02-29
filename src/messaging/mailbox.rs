/// A type that can asynchronously receive messages.
pub trait Mailbox<'a>: Send + Sync {
    type Message: Send + 'a;

    /// Put a message into the mailbox.
    ///
    /// Returns `true` if and only if the message was successfully inserted.
    fn send_msg(&self, msg: Self::Message) -> bool;
}

impl<'a, T: Send + 'a> Mailbox<'a> for crossbeam_channel::Sender<T> {
    type Message = T;

    fn send_msg(&self, msg: Self::Message) -> bool {
        self.try_send(msg).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::messaging::mailbox::Mailbox;
    use crossbeam_channel::bounded;

    #[test]
    fn test_sender_mailbox() {
        let (s, r) = bounded(1);

        assert!(s.send_msg(1));
        assert!(!s.send_msg(2));

        assert_eq!(r.recv().unwrap(), 1);
    }
}
