use crate::parasect::background_loop::BackgroundLoopBehavior::Cancel;
use crossbeam_channel::{bounded, select, Receiver, Sender};
use std::marker::PhantomData;
use std::thread::Scope;

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
pub enum BackgroundLoopBehavior {
    Cancel,
    DontCancel,
}

pub struct BackgroundLoop<'scope> {
    scope_phantom: PhantomData<&'scope ()>,
    cancel_sender: Sender<()>,
}

impl<'scope> BackgroundLoop<'scope> {
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

        scope.spawn(move || loop {
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
            scope_phantom: PhantomData,
        }
    }

    pub fn cancel(&self) {
        let _ = self.cancel_sender.try_send(());
    }
}

impl<'scope> Drop for BackgroundLoop<'scope> {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use crate::parasect::background_loop::BackgroundLoop;
    use crate::parasect::background_loop::BackgroundLoopBehavior::{Cancel, DontCancel};
    use crossbeam_channel::unbounded;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_background_loop() {
        let (send, recv) = unbounded();

        thread::scope(|scope| {
            let _bg = BackgroundLoop::spawn(scope, recv.clone(), |num| {
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
        });

        assert_eq!(recv.recv(), Ok(6));
    }

    #[test]
    fn test_background_loop_initially_full() {
        let (send, recv) = unbounded();

        for i in 0..7 {
            send.send(i).unwrap();
        }

        thread::scope(|scope| {
            let _bg = BackgroundLoop::spawn(scope, recv.clone(), |num| {
                if num == 5 {
                    Cancel
                } else {
                    DontCancel
                }
            });

            thread::sleep(Duration::from_millis(50));
        });

        assert_eq!(recv.recv(), Ok(6));
    }

    #[test]
    fn test_background_cancel_terminates() {
        let (send, recv) = unbounded();

        thread::scope(|scope| {
            let bg = BackgroundLoop::spawn(scope, recv, |_| DontCancel);

            scope.spawn(|| loop {
                match send.send(()) {
                    Ok(_) => {}
                    Err(_) => return,
                };
            });

            bg.cancel();
        });
    }
}
