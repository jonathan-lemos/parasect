use crate::messaging::listener::Listener;
use crate::messaging::mailbox::Mailbox;
use crossbeam_channel::{unbounded, Receiver};
use std::sync::{Arc, RwLock};
use std::thread::Scope;

/// Fans out a receiver into 0 or more receivers. Each message will go to each subscriber.
///
/// We have Pub/Sub at home.
pub struct Fan<'a, T>
where
    T: Send + Clone + 'a,
{
    outputs: Arc<RwLock<Vec<Box<dyn Mailbox<'a, Message = T> + 'a>>>>,
    _message_spreader: Listener<'a, T>,
}

fn instantiation_closure<'a, T: Send + Clone + 'a>() -> (
    impl Fn(T) + Send + 'a,
    Arc<RwLock<Vec<Box<dyn Mailbox<'a, Message = T> + 'a>>>>,
) {
    let outputs = Arc::new(RwLock::new(Vec::<Box<dyn Mailbox<Message = T> + 'a>>::new()));

    let outputs_clone = outputs.clone();
    (
        move |msg: T| {
            for mailbox in outputs_clone.read().unwrap().iter() {
                mailbox.send_msg(msg.clone());
            }
        },
        outputs,
    )
}

impl<T> Fan<'static, T>
where
    T: Send + Clone + 'static,
{
    pub fn new(receiver: Receiver<T>) -> Self {
        let (closure, outputs) = instantiation_closure::<'static>();

        let message_spreader = Listener::spawn(receiver, closure);

        Self {
            outputs,
            _message_spreader: message_spreader,
        }
    }
}

impl<'a, T> Fan<'a, T>
where
    T: Send + Clone + 'a,
{
    #[allow(unused)]
    pub fn new_scoped<'env: 'a>(scope: &'a Scope<'a, 'env>, receiver: Receiver<T>) -> Self {
        let (closure, outputs) = instantiation_closure::<'a>();

        let message_spreader = Listener::spawn_scoped(scope, receiver, closure);

        Self {
            outputs,
            _message_spreader: message_spreader,
        }
    }

    /// Send all messages from this `Fan` to the given `Mailbox`.
    pub fn notify(&self, mailbox: Box<dyn Mailbox<'a, Message = T> + 'a + Send + Sync>) {
        let mut outputs = self.outputs.write().unwrap();
        outputs.push(mailbox);
    }

    /// Make a new `Receiver` that receives all the messages from this `Fan`.
    pub fn subscribe(&self) -> Receiver<T> {
        let (send, recv) = unbounded();
        self.notify(Box::new(send));
        recv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify() {
        let (send, recv) = unbounded();
        let f = Fan::new(recv);

        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();

        f.notify(Box::new(s1));
        f.notify(Box::new(s2));

        send.send_msg(1);
        send.send_msg(2);

        assert_eq!(r1.recv(), Ok(1));
        assert_eq!(r1.recv(), Ok(2));

        assert_eq!(r2.recv(), Ok(1));
        assert_eq!(r2.recv(), Ok(2));
    }

    #[test]
    fn test_subscribe() {
        let (send, recv) = unbounded();
        let f = Fan::new(recv);

        let r1 = f.subscribe();
        let r2 = f.subscribe();

        send.send_msg(1);
        send.send_msg(2);

        assert_eq!(r1.recv(), Ok(1));
        assert_eq!(r1.recv(), Ok(2));

        assert_eq!(r2.recv(), Ok(1));
        assert_eq!(r2.recv(), Ok(2));
    }
}
