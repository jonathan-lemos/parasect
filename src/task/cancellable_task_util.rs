use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use crate::task::cancellable_task::CancellableTask;
use crate::task::cancellable_task_util::CancellationType::*;
use crate::task::cancellable_task_util::MaybeCancelledResult::*;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum CancellationType {
    CancelOthers,
    ContinueOthers
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum MaybeCancelledResult<T> {
    NotCancelled(T),
    Cancelled
}

pub fn execute_parallel_with_cancellation<T, E, TTask, I> (tasks: I) -> Vec<MaybeCancelledResult<T>>
where
    T: Send,
    TTask: CancellableTask<(T, CancellationType), E> + Send,
    I: Iterator<Item = TTask> {
    let mut tasks = tasks.collect::<Vec<TTask>>();
    let task_refs = Mutex::new(tasks.iter_mut().collect::<Vec<&mut TTask>>());
    let cancelling = AtomicBool::new(false);

    thread::scope(|scope| {
        let threads = tasks.into_iter().map(|task| {
            scope.spawn(|| {
                let (val, should_cancel) = task.join();

                if cancelling.load(Ordering::Acquire) {
                    return Cancelled;
                }

                if should_cancel == CancelOthers {
                    cancelling.store(true, Ordering::Release);
                    for task_ref in task_refs.lock().unwrap().into_iter() {
                        // todo: do something about cancellation errors?
                        let _ = task_ref.request_cancellation();
                    }
                }

                NotCancelled(val)
            })
        });

        threads.map(|t| t.join().unwrap()).collect()
    })
}