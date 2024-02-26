/// A type that can asynchronously receive messages.
pub trait Mailbox<'a>: Send + Sync {
    type Message: Send + 'a;

    /// Put a message into the mailbox.
    ///
    /// Returns `true` if and only if the message was successfully inserted.
    fn give_message(&self, msg: Self::Message) -> bool;
}

impl<'a, T: Send + 'a> Mailbox<'a> for crossbeam_channel::Sender<T> {
    type Message = T;

    fn give_message(&self, msg: Self::Message) -> bool {
        self.try_send(msg).is_ok()
    }
}
