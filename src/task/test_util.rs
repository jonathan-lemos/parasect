pub trait ResultLike<T>: Clone {
    fn to_result(&self) -> Option<&T>;
}

impl<T: Clone> ResultLike<T> for Option<&T> {
    fn to_result(&self) -> Option<&T> {
        self.clone()
    }
}

impl<T: Clone> ResultLike<T> for Option<T> {
    fn to_result(&self) -> Option<&T> {
        self.as_ref()
    }
}

impl<T: Clone> ResultLike<T> for T {
    fn to_result(&self) -> Option<&T> {
        Some(&self)
    }
}

macro_rules! assert_result_eq {
    ($a:expr, $b:expr) => {
        assert_eq!($a.to_result(), $b.to_result());
    };
}

use crate::task::cancellable_task::CancellableTask;
use crate::task::test_cancellable_task::TestCancellableTask;
pub(crate) use assert_result_eq;
use crossbeam_channel::bounded;
use std::fmt::Debug;
use std::thread;
use std::time::Duration;

fn assert_notify_same<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    let (s1, r1) = bounded(1);
    let (s2, r2) = bounded(1);

    task.notify_when_done(s1);
    task.notify_when_done(s2);

    assert_eq!(
        r1.recv().unwrap(),
        r2.recv().unwrap(),
        "All notifiers should get the same value."
    );
}

fn assert_wait_idempotent<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    let v1 = task.wait();
    let v2 = task.wait();

    assert_eq!(v1, v2, ".join() was expected to be idempotent");
}

fn assert_cancel_after_wait_ignored<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    let v1 = task.wait();
    task.request_cancellation();
    let v2 = task.wait();
    assert_eq!(
        v1, v2,
        ".request_cancellation() should be ignored after a .join()"
    );
}

fn assert_cancel_idempotent<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    task.request_cancellation();
    let v1 = task.wait();
    task.request_cancellation();
    let v2 = task.wait();

    assert_eq!(v1, v2, ".request_cancellation() should be idempotent");
}

pub fn assert_cancellabletask_invariants<T, C, F>(task_factory: F)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
    F: Fn() -> C,
{
    assert_notify_same(task_factory());
    assert_wait_idempotent(task_factory());
    assert_cancel_after_wait_ignored(task_factory());
    assert_cancel_idempotent(task_factory());
}

fn assert_threaded_notify_idempotent<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    let (v1, v2, v3) = thread::scope(|scope| {
        let (s1, r1) = bounded(1);
        let (s2, r2) = bounded(1);
        let (s3, r3) = bounded(1);

        scope.spawn(|| task.notify_when_done(s1));
        scope.spawn(|| task.notify_when_done(s2));
        scope.spawn(|| task.notify_when_done(s3));

        (r1.recv().unwrap(), r2.recv().unwrap(), r3.recv().unwrap())
    });

    assert_eq!(v1, v2, ".wait() should be idempotent");
    assert_eq!(v2, v3, ".wait() should be idempotent");
}

fn assert_threaded_waits_idempotent<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    let (v1, v2, v3) = thread::scope(|scope| {
        let t1 = scope.spawn(|| task.wait());
        let t2 = scope.spawn(|| task.wait());
        let t3 = scope.spawn(|| task.wait());

        (t1.join().unwrap(), t2.join().unwrap(), t3.join().unwrap())
    });

    assert_eq!(v1, v2, ".wait() should be idempotent");
    assert_eq!(v2, v3, ".wait() should be idempotent");
}

fn assert_threaded_cancel_wait_safe<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    let (v1, v2, v3) = thread::scope(|scope| {
        scope.spawn(|| task.request_cancellation());
        let t1 = scope.spawn(|| task.wait());
        scope.spawn(|| task.request_cancellation());
        let t2 = scope.spawn(|| task.wait());
        scope.spawn(|| task.request_cancellation());
        let t3 = scope.spawn(|| task.wait());
        scope.spawn(|| task.request_cancellation());

        (t1.join().unwrap(), t2.join().unwrap(), t3.join().unwrap())
    });

    assert_eq!(v1, v2, ".join() should be idempotent");
    assert_eq!(v2, v3, ".join() should be idempotent");
}

pub fn assert_cancellabletask_thread_safe<T, C, F>(task_factory: F)
where
    T: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
    F: Fn() -> C,
{
    assert_threaded_notify_idempotent(task_factory());
    assert_threaded_waits_idempotent(task_factory());
    assert_threaded_cancel_wait_safe(task_factory());
}

pub fn assert_higher_order_notify_before<A, B, C>(
    value: A,
    expected_eq: B,
    inner: TestCancellableTask<A>,
    task: C,
) where
    A: Eq + Debug + Send + Sync + Clone + 'static,
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
{
    let (s, r) = bounded(1);

    inner.send(value);
    task.notify_when_done(s);
    assert_eq!(r.recv().unwrap(), Some(expected_eq));
}

pub fn assert_higher_order_notify_after<A, B, C>(
    value: A,
    expected_eq: B,
    inner: TestCancellableTask<A>,
    task: C,
) where
    A: Eq + Debug + Send + Sync + Clone + 'static,
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
{
    let (s, r) = bounded(1);
    task.notify_when_done(s);

    inner.send(value);

    assert_eq!(r.recv().unwrap(), Some(expected_eq));
}

pub fn assert_higher_order_wait_before<A, B, C>(
    value: A,
    expected_eq: B,
    inner: TestCancellableTask<A>,
    task: C,
) where
    A: Eq + Debug + Send + Sync + Clone + 'static,
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
{
    inner.send(value);
    assert_eq!(task.wait(), Some(expected_eq));
}

pub fn assert_higher_order_wait_after<A, B, C>(
    value: A,
    expected_eq: B,
    inner: TestCancellableTask<A>,
    task: C,
) where
    A: Eq + Debug + Send + Sync + Clone + 'static,
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
{
    let v = thread::scope(|scope| {
        let handle = scope.spawn(|| task.wait());
        inner.block_for_wait(Duration::from_secs(3), ".wait() never returned");
        inner.send(value);

        handle.join().unwrap()
    });

    assert_eq!(v, Some(expected_eq));
}

pub fn assert_higher_order_cancellabletask_eq<A, B, C, F>(
    value: A,
    expected_eq: B,
    higher_order_task_factory: F,
) where
    A: Eq + Debug + Send + Sync + Clone + 'static,
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
    F: Fn() -> (TestCancellableTask<A>, C),
{
    let (tt, t) = higher_order_task_factory();
    assert_higher_order_notify_before(value.clone(), expected_eq.clone(), tt, t);

    let (tt, t) = higher_order_task_factory();
    assert_higher_order_notify_after(value.clone(), expected_eq.clone(), tt, t);

    let (tt, t) = higher_order_task_factory();
    assert_higher_order_wait_before(value.clone(), expected_eq.clone(), tt, t);

    let (tt, t) = higher_order_task_factory();
    assert_higher_order_wait_after(value.clone(), expected_eq.clone(), tt, t);
}

pub fn assert_cancel_before_no_deadlock<B, C>(task: C)
where
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
{
    task.request_cancellation();
    task.wait();
}

pub fn assert_cancel_after_no_deadlock<A, B, C>(inner: TestCancellableTask<A>, task: C)
where
    A: Eq + Debug + Send + Sync + Clone + 'static,
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
{
    thread::scope(|scope| {
        let handle = scope.spawn(|| task.wait());
        inner.block_for_wait(Duration::from_secs(3), ".wait() was never called");
        inner.request_cancellation();

        handle.join().unwrap();
    });
}

pub fn assert_higher_order_cancellabletask_cancel_no_deadlock<A, B, C, F>(
    higher_order_task_factory: F,
) where
    A: Eq + Debug + Send + Sync + Clone + 'static,
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
    F: Fn() -> (TestCancellableTask<A>, C),
{
    let (_tt, t) = higher_order_task_factory();
    assert_cancel_before_no_deadlock(t);

    let (tt, t) = higher_order_task_factory();
    assert_cancel_after_no_deadlock(tt, t);
}

pub fn assert_higher_order_cancellabletask_invariants<A, B, C, F>(
    value: A,
    expected_eq: B,
    higher_order_task_factory: F,
) where
    A: Eq + Debug + Send + Sync + Clone + 'static,
    B: Eq + Debug + Send + Sync + Clone + 'static,
    C: CancellableTask<B>,
    F: (Fn() -> (TestCancellableTask<A>, C)),
{
    assert_higher_order_cancellabletask_eq(value, expected_eq, &higher_order_task_factory);
    assert_higher_order_cancellabletask_cancel_no_deadlock(&higher_order_task_factory);
}
