use crate::threading::once_reactor::{OnceReactor, OnceSender};

/// A type that can be asynchronously notified with a message.
pub trait Notifiable {
    type Message: Send;

    fn notify(&self, msg: Self::Message) -> bool;
}

impl<T: Send> Notifiable for crossbeam_channel::Sender<T> {
    type Message = T;

    fn notify(&self, msg: Self::Message) -> bool {
        self.send(msg).is_ok()
    }
}

impl<T: Send> Notifiable for OnceSender<T> {
    type Message = T;

    fn notify(&self, msg: Self::Message) -> bool {
        self.send(msg)
    }
}

impl<T: Send> Notifiable for OnceReactor<T> {
    type Message = T;

    fn notify(&self, msg: Self::Message) -> bool {
        self.sender().notify(msg)
    }
}
