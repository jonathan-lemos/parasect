use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Send a message to the `receiver()` every time the given `interval` passes.
///
/// A maximum of two messages can be queued to prevent the buffer from overflowing for a slow receiver.
pub struct PeriodicNotifier {
    recv: Receiver<()>,
    cancelled: Arc<AtomicBool>,
}

impl PeriodicNotifier {
    pub fn new(interval: Duration) -> Self {
        let (send, recv) = bounded(2);
        let cancelled = Arc::new(AtomicBool::new(false));

        let cancelled_clone = cancelled.clone();
        thread::spawn(move || {
            while !cancelled_clone.load(Ordering::SeqCst) {
                let _ = send.try_send(());
                thread::sleep(interval);
            }
        });

        Self { cancelled, recv }
    }

    pub fn stop(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn receiver(&self) -> Receiver<()> {
        self.recv.clone()
    }

    pub fn active(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst)
    }
}

impl Drop for PeriodicNotifier {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI64;

    #[test]
    fn test_periodic_notifier() {
        let counter = Arc::new(AtomicI64::new(0));

        let pn = PeriodicNotifier::new(Duration::from_millis(1000));

        let counter_clone = counter.clone();
        let recv = pn.receiver();
        let t = thread::spawn(move || {
            while let Ok(_) = recv.recv() {
                counter_clone.fetch_add(1, Ordering::Relaxed);
            }
        });

        thread::sleep(Duration::from_millis(3500));
        assert!(pn.active());
        pn.stop();
        assert!(!pn.active());

        t.join().unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }
}
