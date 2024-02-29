use crate::messaging::listener::Listener;
use crate::messaging::mailbox::Mailbox;
use crossbeam_channel::Receiver;
use std::thread::Scope;

/// Pipes messages from a `Receiver` to a `Mailbox`.
///
/// Only does so until it's dropped.
pub struct Pipe<'a, T>
where
    T: Send + 'a,
{
    listener: Listener<'a, T>,
}

impl<T> Pipe<'static, T>
where
    T: Send + 'static,
{
    /// Create a `Pipe` that forwards messages from `receiver` to `mailbox`.
    pub fn new(
        receiver: Receiver<T>,
        mailbox: impl Mailbox<'static, Message = T> + 'static,
    ) -> Self {
        Self {
            listener: Listener::spawn(receiver, move |msg| {
                mailbox.send_msg(msg);
            }),
        }
    }
}

impl<'a, T> Pipe<'a, T>
where
    T: Send + 'a,
{
    /// Create a `Pipe` that only lives as long as the `scope` used to create it. It forwards messages from `receiver` to `mailbox`.
    pub fn new_scoped<'env: 'a>(
        scope: &'a Scope<'a, 'env>,
        receiver: Receiver<T>,
        mailbox: impl Mailbox<'a, Message = T> + 'a,
    ) -> Self {
        Self {
            listener: Listener::spawn_scoped(scope, receiver, move |msg| {
                mailbox.send_msg(msg);
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::messaging::pipe::Pipe;
    use crossbeam_channel::unbounded;
    use std::thread;

    #[test]
    fn test_pipe() {
        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();

        let p = Pipe::new(r1, s2);

        s1.send(1).unwrap();
        assert_eq!(r2.recv().unwrap(), 1);
    }

    #[test]
    fn test_pipe_scoped() {
        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();

        let (s1c, r1c) = (s1.clone(), r1.clone());
        let (s2c, r2c) = (s2.clone(), r2.clone());
        thread::scope(move |scope| {
            let p = Pipe::new_scoped(scope, r1c, s2c);

            s1c.send(1).unwrap();
            assert_eq!(r2c.recv().unwrap(), 1);
        });

        s1.send(1).unwrap();
        assert!(r2.try_recv().is_err());
    }
}
