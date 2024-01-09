use std::thread;
use crate::task::cancellable_task::CancellableTask;
use crate::task::cancellable_task_util::CancellationType::*;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum CancellationType {
    CancelOthers,
    ContinueOthers
}

pub fn execute_parallel_with_cancellation<T, TTask, I> (tasks: I) -> Vec<Option<T>>
where
    T: Send + Sync,
    TTask: CancellableTask<(T, CancellationType)> + Send,
    I: Iterator<Item = TTask> {
    let tasks = tasks.collect::<Vec<TTask>>();

    thread::scope(|scope| {
        let threads = tasks.iter().map(|task| {
            scope.spawn(|| {
                let should_cancel = match task.join() {
                    Some((_, b)) => b,
                    None => return
                };

                if should_cancel == &CancelOthers {
                    for task in tasks.iter() {
                        task.request_cancellation()
                    }
                }
            })
        });

        for thread in threads {
            thread.join();
        }
    });

    tasks.into_iter().map(|x| x.join_into().map(|y| y.0)).collect()
}