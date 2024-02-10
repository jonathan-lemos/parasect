use crate::parasect::background_loop::BackgroundLoopBehavior::Cancel;
use crossbeam_channel::{bounded, select, Receiver, Sender};
use std::thread::{Scope, ScopedJoinHandle};

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
pub enum BackgroundLoopBehavior {
    Cancel,
    DontCancel,
}

pub struct BackgroundLoop<'scope> {
    thread: ScopedJoinHandle<'scope, ()>,
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
            thread,
            cancel_sender,
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
