use crate::messaging::mailbox::Mailbox;
use crate::task::cancellable_task::CancellableTask;
use crossbeam_channel::bounded;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

enum Inner<T>
where
    T: Send + Clone,
{
    Waiters(Vec<Box<dyn Mailbox<'static, Message = T>>>),
    Value(T),
}

/// A value that is initialized a single time asynchronously.
pub struct AsyncValue<T>
where
    T: Send + Clone,
{
    inner: Arc<RwLock<Inner<T>>>,
}

impl<T> Debug for AsyncValue<T>
where
    T: Send + Clone + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self.inner.read().unwrap().deref() {
            Inner::Waiters(ws) => format!("AsyncValue({} awaiting)", ws.len()),
            Inner::Value(v) => format!("AsyncValue(done {:?})", v),
        })
    }
}

impl<T> PartialEq for AsyncValue<T>
where
    T: Send + Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }

        match (
            self.inner.read().unwrap().deref(),
            other.inner.read().unwrap().deref(),
        ) {
            (Inner::Value(a), Inner::Value(b)) => a.eq(b),
            _ => false,
        }
    }
}

impl<T> Eq for AsyncValue<T> where T: Send + Clone + Eq {}

impl<T> AsyncValue<T>
where
    T: Send + Clone + 'static,
{
    /// Creates a new AsyncValue that's awaiting initialization through `send()`.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::Waiters(Vec::new()))),
        }
    }

    fn get_value_if_exists(&self) -> Option<T> {
        let read = self.inner.read().unwrap();

        match read.deref() {
            Inner::Value(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// Sends the inner value to given `Notifiable` when this `AsyncValue` is initialized.
    ///
    /// The Sender should be guaranteed to have enough capacity to hold a value, otherwise `send()` will block for the full Sender.
    ///
    /// If the `AsyncValue` is already initialized, immediately sends the value.
    pub fn notify(&self, notifiable: impl Mailbox<'static, Message = T> + 'static) {
        if let Some(v) = self.get_value_if_exists() {
            notifiable.send_msg(v);
            return;
        }

        let mut write = self.inner.write().unwrap();

        match write.deref_mut() {
            Inner::Value(v) => {
                notifiable.send_msg(v.clone());
            }
            Inner::Waiters(ws) => ws.push(Box::new(notifiable)),
        };
    }

    /// Initializes the `AsyncValue`.
    ///
    /// If it is already initialized, does nothing.
    pub fn send(&self, value: T) -> bool {
        if let Some(_) = self.get_value_if_exists() {
            return false;
        }

        let mut write = self.inner.write().unwrap();

        {
            let ws = match write.deref_mut() {
                Inner::Value(_) => return false,
                Inner::Waiters(ws) => ws,
            };

            while let Some(w) = ws.pop() {
                w.send_msg(value.clone());
            }
        }

        *write = Inner::Value(value);
        true
    }

    /// Blocks until this `AsyncValue` is initialized, then returns the value.
    pub fn wait(&self) -> T {
        let (send, recv) = bounded(1);
        self.notify(send);
        recv.recv().unwrap()
    }
}

impl<T> From<T> for AsyncValue<T>
where
    T: Send + Clone + 'static,
{
    fn from(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::Value(value))),
        }
    }
}

impl<T> Clone for AsyncValue<T>
where
    T: Send + Clone + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Send + Sync + Clone + 'static> CancellableTask<T> for AsyncValue<Option<T>> {
    fn notify_when_done(&self, mailbox: impl Mailbox<'static, Message = Option<T>> + 'static) {
        self.notify(mailbox);
    }

    fn request_cancellation(&self) -> () {
        self.send(None);
    }
}

impl<T: Send + Sync + Clone + 'static> Mailbox<'static> for AsyncValue<T> {
    type Message = T;

    fn send_msg(&self, msg: Self::Message) -> bool {
        self.send(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::test_util::{
        assert_cancellabletask_invariants, assert_cancellabletask_thread_safe,
    };
    use proptest::proptest;
    use std::thread;

    #[test]
    pub fn test_from() {
        let a = AsyncValue::from(69);

        let (s, r) = bounded(1);

        a.notify(s);

        assert_eq!(r.recv().unwrap(), 69);
        assert_eq!(a.wait(), 69);
    }

    #[test]
    pub fn test_notify_notifies() {
        let a = AsyncValue::new();

        let (s1, r1) = bounded(1);
        let (s2, r2) = bounded(1);

        a.notify(s1);
        a.notify(s2);

        a.send(69);

        assert_eq!(r1.recv().unwrap(), 69);
        assert_eq!(r2.recv().unwrap(), 69);
    }

    #[test]
    pub fn test_mailbox() {
        let a = AsyncValue::new();

        let (s1, r1) = bounded(1);
        let (s2, r2) = bounded(1);

        a.notify(s1);
        a.notify(s2);

        a.send_msg(69);

        assert_eq!(r1.recv().unwrap(), 69);
        assert_eq!(r2.recv().unwrap(), 69);
    }

    #[test]
    pub fn test_notify_after_send() {
        let a = AsyncValue::new();

        let (s1, r1) = bounded(1);
        let (s2, r2) = bounded(1);

        a.send(69);

        a.notify(s1);
        a.notify(s2);

        assert_eq!(r1.recv().unwrap(), 69);
        assert_eq!(r2.recv().unwrap(), 69);
    }

    #[test]
    pub fn test_wait_blocks_until_send() {
        let a = AsyncValue::new();

        let (v1, v2) = thread::scope(|scope| {
            let r1 = scope.spawn(|| a.wait());
            let r2 = scope.spawn(|| a.wait());

            a.send(69);
            (r1.join().unwrap(), r2.join().unwrap())
        });

        assert_eq!(v1, 69);
        assert_eq!(v2, 69);
    }

    #[test]
    pub fn test_wait_noblock_after_send() {
        let a = AsyncValue::new();

        a.send(69);
        let v1 = a.wait();
        let v2 = a.wait();

        assert_eq!(v1, 69);
        assert_eq!(v2, 69);
    }

    #[test]
    pub fn test_clone() {
        let a = AsyncValue::new();
        let b = a.clone();

        a.send(69);

        assert_eq!(a.wait(), 69);
        assert_eq!(b.wait(), 69);
    }

    #[test]
    pub fn test_send_idempotent() {
        let a = AsyncValue::new();

        assert!(a.send(69));
        assert!(!a.send(70));
    }

    #[test]
    pub fn test_cancellabletask_invariants() {
        assert_cancellabletask_invariants(|| AsyncValue::from(Some(69)));
    }

    proptest! {
        #[test]
        fn notify_concurrent(i in 1..10000) {
            let a = AsyncValue::new();

            let (s1, r1) = bounded(1);
            let (s2, r2) = bounded(1);
            let (s3, r3) = bounded(1);
            let (s4, r4) = bounded(1);

            let vs = thread::scope(|scope| {
                let t1 = scope.spawn(|| a.notify(s1));
                let t2 = scope.spawn(|| a.notify(s2));
                scope.spawn(|| a.send(i));
                let t3 = scope.spawn(|| a.notify(s3));
                let t4 = scope.spawn(|| a.notify(s4));

                [t1, t2, t3, t4].into_iter().for_each(|x| x.join().unwrap());
                [r1, r2, r3, r4].map(|x| x.recv().unwrap())
            });

            assert!(vs.into_iter().all(|x| x == i));
        }

        #[test]
        fn send_concurrent(i in 1..10000) {
            let a = AsyncValue::new();

            thread::scope(|scope| {
                scope.spawn(|| a.send(i));
                scope.spawn(|| a.send(i));
                scope.spawn(|| a.send(i));
                scope.spawn(|| a.send(i));
            });

            assert_eq!(a.wait(), i);
        }

        #[test]
        fn wait_concurrent(i in 1..10000) {
            let a = AsyncValue::new();

            let vs = thread::scope(|scope| {
                let t1 = scope.spawn(|| a.wait());
                let t2 = scope.spawn(|| a.wait());
                scope.spawn(|| a.send(i));
                let t3 = scope.spawn(|| a.wait());
                let t4 = scope.spawn(|| a.wait());

                [t1, t2, t3, t4].map(|x| x.join().unwrap())
            });

            assert!(vs.into_iter().all(|x| x == i));
        }

        #[test]
        fn test_threadsafe(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| AsyncValue::from(Some(i)));
        }
    }
}
