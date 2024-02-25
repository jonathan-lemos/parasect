use crate::threading::background_loop::BackgroundLoop;
use crate::threading::background_loop::BackgroundLoopBehavior::DontCancel;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};

/// Fans out a receiver into 0 or more receivers. Each message will go to each subscriber.
///
/// We have Pub/Sub at home.
pub struct Fan<T>
where
    T: Send + Clone,
{
    outputs: Arc<RwLock<Vec<Sender<T>>>>,
    _thread: BackgroundLoop,
    receivers: Mutex<Vec<Pin<Box<Receiver<T>>>>>,
}

impl<T> Fan<T>
where
    T: Send + Clone + 'static,
{
    pub fn new(receiver: Receiver<T>) -> Self {
        let outputs = Arc::new(RwLock::new(Vec::new()));

        let outputs_clone = outputs.clone();

        Self {
            outputs,
            receivers: Mutex::new(Vec::new()),
            _thread: BackgroundLoop::spawn(receiver, move |msg| {
                for snd in outputs_clone.read().unwrap().iter() {
                    let _ = snd.send(msg.clone());
                }
                DontCancel
            }),
        }
    }

    /// Returns a Receiver that receives all the messages given to this Fan.
    pub fn subscribe(&self) -> &Receiver<T> {
        let (send, recv) = unbounded();

        {
            let mut outputs = self.outputs.write().unwrap();
            outputs.push(send);
        }

        let mut receivers = self.receivers.lock().unwrap();
        receivers.push(Box::pin(recv));

        let pin_ref = receivers.last().unwrap();
        let receiver_ref = pin_ref.deref();

        // unsafe{} is necessary to get a reference to the data in the mutex.
        // this is safe because the underlying pin prevents the data from moving,
        // and this struct will never remove a value from the vec,
        // so the reference shouldn't be invalidated until this struct drops,
        // but the returned reference's lifetime is tied to this struct
        unsafe { (receiver_ref as *const Receiver<T>).as_ref().unwrap() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fan() {
        let (send, recv) = unbounded();

        let f = Fan::new(recv);

        let s1 = f.subscribe();
        let s2 = f.subscribe();

        send.send(1).unwrap();
        send.send(2).unwrap();

        assert_eq!(s1.recv(), Ok(1));
        assert_eq!(s1.recv(), Ok(2));

        assert_eq!(s2.recv(), Ok(1));
        assert_eq!(s2.recv(), Ok(2));
    }
}
