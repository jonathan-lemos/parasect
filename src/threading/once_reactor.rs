use crate::threading::background_loop::BackgroundLoop;
use crate::threading::background_loop::BackgroundLoopBehavior::Cancel;
use crate::threading::single_use_cell::SingleUseCell;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::{Arc, RwLock};

/// A sender that can only take a value once.
#[derive(Debug)]
pub struct OnceSender<T: Send + 'static> {
    send: Arc<RwLock<(Sender<T>, bool)>>,
}

impl<T: Send + 'static> OnceSender<T> {
    fn new() -> (Self, Receiver<T>) {
        let (send, recv) = bounded(1);
        (
            Self {
                send: Arc::new(RwLock::new((send, false))),
            },
            recv,
        )
    }

    /// Sends a value, if a value was not already sent.
    ///
    /// Returns true if and only if the message was successfully sent.
    pub fn send(&self, value: T) -> bool {
        {
            let read = self.send.read().unwrap();
            if read.1 {
                return false;
            }
        }

        let mut write = self.send.write().unwrap();
        write.0.send(value).unwrap();
        write.1 = true;
        return true;
    }
}

impl<T: Send + 'static> Clone for OnceSender<T> {
    fn clone(&self) -> Self {
        Self {
            send: self.send.clone(),
        }
    }
}

/// Waits for a message, then runs a function in response to it.
///
/// All subsequent messages are ignored.
pub struct OnceReactor<T>
where
    T: Send + 'static,
{
    send: OnceSender<T>,
    listener: BackgroundLoop,
}

impl<T> OnceReactor<T>
where
    T: Send + 'static,
{
    pub fn new(handler: impl FnOnce(T) -> () + Send + 'static) -> Self {
        let (send, recv) = OnceSender::new();
        let handler_cell = SingleUseCell::new(handler);
        Self {
            send,
            listener: BackgroundLoop::spawn(recv, move |msg| {
                handler_cell.take().unwrap()(msg);
                Cancel
            }),
        }
    }

    pub fn sender(&self) -> OnceSender<T> {
        self.send.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::threading::once_reactor::OnceSender;

    #[test]
    fn test_once_sender() {
        let (send, recv) = OnceSender::new();

        assert_eq!(send.send(69), true);
        assert_eq!(send.send(70), false);

        assert_eq!(recv.try_recv(), Ok(69));
        assert!(recv.try_recv().is_err());
    }
}
