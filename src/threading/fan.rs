use crate::threading::actor::Actor;
use crate::threading::mailbox::Mailbox;
use std::sync::{Arc, RwLock};

/// Fans out a receiver into 0 or more receivers. Each message will go to each subscriber.
///
/// We have Pub/Sub at home.
pub struct Fan<T>
where
    T: Send + Clone + 'static,
{
    outputs: Arc<RwLock<Vec<Box<dyn Mailbox<Message = T> + 'static + Send + Sync>>>>,
    message_spreader: Actor<T>,
}

impl<T> Fan<T>
where
    T: Send + Clone + 'static,
{
    pub fn new() -> Self {
        let outputs = Arc::new(RwLock::new(Vec::<
            Box<dyn Mailbox<Message = T> + 'static + Send + Sync>,
        >::new()));

        let outputs_clone = outputs.clone();
        let message_spreader = Actor::spawn(move |msg: T| {
            for mailbox in outputs_clone.read().unwrap().iter() {
                mailbox.give_message(msg.clone());
            }
        });

        Self {
            outputs,
            message_spreader,
        }
    }

    pub fn mailbox(&self) -> impl Mailbox<Message = T> {
        self.message_spreader.mailbox()
    }

    /// Returns a Receiver that receives all the messages given to this Fan.
    pub fn subscribe(&self, mailbox: Box<dyn Mailbox<Message = T> + 'static + Send + Sync>) {
        let mut outputs = self.outputs.write().unwrap();
        outputs.push(mailbox);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn test_fan() {
        let f = Fan::new();

        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();

        f.subscribe(Box::new(s1));
        f.subscribe(Box::new(s2));

        f.mailbox().give_message(1);
        f.mailbox().give_message(2);

        assert_eq!(r1.recv(), Ok(1));
        assert_eq!(r1.recv(), Ok(2));

        assert_eq!(r2.recv(), Ok(1));
        assert_eq!(r2.recv(), Ok(2));
    }
}
