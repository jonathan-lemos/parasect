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
pub(crate) use assert_result_eq;
use std::fmt::Debug;
use std::thread;

fn assert_join_idempotent<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync,
    C: CancellableTask<T>,
{
    let v1 = task.join();
    let v2 = task.join();

    assert_eq!(v1, v2, ".join() was expected to be idempotent");
}

fn assert_join_clone_equals_join_into<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone,
    C: CancellableTask<T>,
{
    let v1 = task.join_clone();
    let v2 = task.join_into();

    assert_eq!(v1, v2, ".join_clone() was expected to equal .join_into()");
}

fn assert_cancel_after_join_ignored<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync,
    C: CancellableTask<T>,
{
    let v1 = task.join();
    task.request_cancellation();
    let v2 = task.join();
    assert_eq!(
        v1, v2,
        ".request_cancellation() should be ignored after a .join()"
    );
}

fn assert_join_clone_equals_join_1<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone,
    C: CancellableTask<T>,
{
    let v1 = task.join_clone();
    let v2 = task.join();

    assert_eq!(
        v1.as_ref(),
        v2,
        ".join_clone() should be a clone of .join()"
    );
}

fn assert_join_clone_equals_join_2<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync + Clone,
    C: CancellableTask<T>,
{
    let v1 = task.join();
    let v2 = task.join_clone();

    assert_eq!(
        v1,
        v2.as_ref(),
        ".join_clone() should be a clone of .join()"
    );
}

fn assert_cancel_idempotent<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync,
    C: CancellableTask<T>,
{
    task.request_cancellation();
    let v1 = task.join();
    task.request_cancellation();
    let v2 = task.join();

    assert_eq!(v1, v2, ".request_cancellation() should be idempotent");
}

pub fn assert_cancellabletask_invariants_noclone<T, C, F>(task_factory: F)
where
    T: Eq + Debug + Send + Sync,
    C: CancellableTask<T>,
    F: Fn() -> C,
{
    assert_join_idempotent(task_factory());
    assert_cancel_after_join_ignored(task_factory());
    assert_cancel_idempotent(task_factory());
}

pub fn assert_cancellabletask_invariants<T, C, F>(task_factory: F)
where
    T: Eq + Debug + Send + Sync + Clone,
    C: CancellableTask<T>,
    F: Fn() -> C,
{
    assert_join_clone_equals_join_into(task_factory());
    assert_join_clone_equals_join_1(task_factory());
    assert_join_clone_equals_join_2(task_factory());
    assert_cancellabletask_invariants_noclone(task_factory);
}

fn assert_threaded_joins_idempotent<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync,
    C: CancellableTask<T>,
{
    let (v1, v2, v3) = thread::scope(|scope| {
        let t1 = scope.spawn(|| task.join());
        let t2 = scope.spawn(|| task.join());
        let t3 = scope.spawn(|| task.join());

        (t1.join().unwrap(), t2.join().unwrap(), t3.join().unwrap())
    });

    assert_eq!(v1, v2, ".join() should be idempotent");
    assert_eq!(v2, v3, ".join() should be idempotent");
}

fn assert_threaded_cancel_join_safe<T, C>(task: C)
where
    T: Eq + Debug + Send + Sync,
    C: CancellableTask<T>,
{
    let (v1, v2, v3) = thread::scope(|scope| {
        scope.spawn(|| task.request_cancellation());
        let t1 = scope.spawn(|| task.join());
        scope.spawn(|| task.request_cancellation());
        let t2 = scope.spawn(|| task.join());
        scope.spawn(|| task.request_cancellation());
        let t3 = scope.spawn(|| task.join());
        scope.spawn(|| task.request_cancellation());

        (t1.join().unwrap(), t2.join().unwrap(), t3.join().unwrap())
    });

    assert_eq!(v1, v2, ".join() should be idempotent");
    assert_eq!(v2, v3, ".join() should be idempotent");
}

pub fn assert_cancellabletask_thread_safe<T, C, F>(task_factory: F)
where
    T: Eq + Debug + Send + Sync + Clone,
    C: CancellableTask<T>,
    F: Fn() -> C,
{
    assert_threaded_joins_idempotent(task_factory());
    assert_threaded_cancel_join_safe(task_factory());
}
