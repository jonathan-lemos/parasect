use crate::threading::background_loop::BackgroundLoopBehavior::Cancel;
use crossbeam_channel::{bounded, select, Receiver, Sender};
use std::thread;
use std::thread::{JoinHandle, Scope, ScopedJoinHandle};

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
pub enum BackgroundLoopBehavior {
    Cancel,
    DontCancel,
}

/// A background thread that continually reads messages from a Receiver and executes the given function on them.
pub struct BackgroundLoop {
    thread: JoinHandle<()>,
    cancel_sender: Sender<()>,
}

impl BackgroundLoop {
    /// Executes the given payload for each message that enters the given receiver.
    ///
    /// Terminates on `cancel()` or if receiving fails (e.g. if the channel disconnects).
    pub fn spawn<T, FPayload>(receiver: Receiver<T>, payload: FPayload) -> Self
    where
        T: Send + 'static,
        FPayload: Fn(T) -> BackgroundLoopBehavior + Send + 'static,
    {
        let (cancel_sender, cancel_receiver) = bounded(1);

        let thread = thread::spawn(move || loop {
            select! {
                recv(cancel_receiver) -> _ => return,
                recv(receiver) -> val => {
                    if let Ok(v) = val {
                        if payload(v) == Cancel {
                            return;
                        }
                    } else {
                        return;
                    }
                }
            }
        });

        Self {
            cancel_sender,
            thread,
        }
    }

    /// Stops the background loop.
    pub fn cancel(&self) {
        let _ = self.cancel_sender.try_send(());
    }

    /// `true` if the BackgroundLoop is still processing messages.
    pub fn running(&self) -> bool {
        !self.thread.is_finished()
    }
}

impl Drop for BackgroundLoop {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// A BackgroundLoop that works within a thread scope.
pub struct ScopedBackgroundLoop<'scope> {
    thread: ScopedJoinHandle<'scope, ()>,
    cancel_sender: Sender<()>,
}

impl<'scope> ScopedBackgroundLoop<'scope> {
    /// Executes the given payload for each message that enters the given receiver.
    ///
    /// Terminates on `cancel()` or if receiving fails (e.g. if the channel disconnects).
    pub fn spawn<'env, T, FPayload>(
        scope: &'scope Scope<'scope, 'env>,
        receiver: Receiver<T>,
        payload: FPayload,
    ) -> Self
    where
        'env: 'scope,
        T: Send + 'scope,
        FPayload: Fn(T) -> BackgroundLoopBehavior + Send + 'scope,
    {
        let (cancel_sender, cancel_receiver) = bounded(1);

        let thread = scope.spawn(move || loop {
            select! {
                recv(cancel_receiver) -> _ => return,
                recv(receiver) -> val => {
                    if let Ok(v) = val {
                        if payload(v) == Cancel {
                            return;
                        }
                    } else {
                        return;
                    }
                }
            }
        });

        Self {
            cancel_sender,
            thread,
        }
    }

    /// Stops the background loop.
    pub fn cancel(&self) {
        let _ = self.cancel_sender.try_send(());
    }

    /// `true` if the BackgroundLoop is still processing messages.
    pub fn running(&self) -> bool {
        !self.thread.is_finished()
    }
}

impl<'scope> Drop for ScopedBackgroundLoop<'scope> {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use crate::threading::background_loop::BackgroundLoopBehavior::{Cancel, DontCancel};
    use crate::threading::background_loop::{BackgroundLoop, ScopedBackgroundLoop};
    use crossbeam_channel::unbounded;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_background_loop() {
        let (send, recv) = unbounded();

        let bg = BackgroundLoop::spawn(
            recv.clone(),
            |num| {
                if num == 5 {
                    Cancel
                } else {
                    DontCancel
                }
            },
        );

        for i in 0..7 {
            send.send(i).unwrap();
        }

        thread::sleep(Duration::from_millis(50));
        assert!(!bg.running());

        assert_eq!(recv.recv(), Ok(6));
    }

    #[test]
    fn test_background_loop_initially_full() {
        let (send, recv) = unbounded();

        for i in 0..7 {
            send.send(i).unwrap();
        }

        let bg = BackgroundLoop::spawn(
            recv.clone(),
            |num| {
                if num == 5 {
                    Cancel
                } else {
                    DontCancel
                }
            },
        );

        thread::sleep(Duration::from_millis(50));
        assert!(!bg.running());

        assert_eq!(recv.recv(), Ok(6));
    }

    #[test]
    fn test_background_cancel_terminates() {
        let (send, recv) = unbounded();

        let bg = BackgroundLoop::spawn(recv, |_| DontCancel);

        thread::spawn(move || loop {
            match send.send(()) {
                Ok(_) => {}
                Err(_) => return,
            };
        });

        assert!(bg.running());
        bg.cancel();
        thread::sleep(Duration::from_millis(50));
        assert!(!bg.running());
    }

    #[test]
    fn test_scoped_background_loop() {
        let (send, recv) = unbounded();

        thread::scope(|scope| {
            let bg = ScopedBackgroundLoop::spawn(scope, recv.clone(), |num| {
                if num == 5 {
                    Cancel
                } else {
                    DontCancel
                }
            });

            for i in 0..7 {
                send.send(i).unwrap();
            }

            thread::sleep(Duration::from_millis(50));
            assert!(!bg.running());
        });

        assert_eq!(recv.recv(), Ok(6));
    }

    #[test]
    fn test_scoped_background_loop_initially_full() {
        let (send, recv) = unbounded();

        for i in 0..7 {
            send.send(i).unwrap();
        }

        thread::scope(|scope| {
            let bg = ScopedBackgroundLoop::spawn(scope, recv.clone(), |num| {
                if num == 5 {
                    Cancel
                } else {
                    DontCancel
                }
            });

            thread::sleep(Duration::from_millis(50));
            assert!(!bg.running());
        });

        assert_eq!(recv.recv(), Ok(6));
    }

    #[test]
    fn test_scoped_background_cancel_terminates() {
        let (send, recv) = unbounded();

        thread::scope(|scope| {
            let bg = ScopedBackgroundLoop::spawn(scope, recv, |_| DontCancel);

            scope.spawn(|| loop {
                match send.send(()) {
                    Ok(_) => {}
                    Err(_) => return,
                };
            });

            assert!(bg.running());
            bg.cancel();
            thread::sleep(Duration::from_millis(50));
            assert!(!bg.running());
        });
    }
}
