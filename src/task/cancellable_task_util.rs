use crate::task::cancellable_task::CancellableTask;
use crate::task::cancellable_task_util::CancellationType::*;
use std::sync::Arc;
use std::thread;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum CancellationType {
    CancelOthers,
    ContinueOthers,
}

/// Executes a sequence of tasks in parallel, cancelling any remaining tasks on request.
///
/// The returned CancellationType of a CancellableTask should be set to CancelOthers to cancel all remaining tasks.
pub fn execute_parallel_cancellable<T, TTask, I>(tasks: I) -> Vec<Option<T>>
where
    T: Send + Sync,
    TTask: CancellableTask<(T, CancellationType)> + Send,
    I: Iterator<Item = TTask>,
{
    let tasks = tasks.collect::<Vec<TTask>>();

    thread::scope(|scope| {
        let threads = tasks.iter().map(|task| {
            scope.spawn(|| {
                let should_cancel = match task.join() {
                    Some(arc) => arc.as_ref().1.clone(),
                    None => return,
                };

                if should_cancel == CancelOthers {
                    for task in tasks.iter() {
                        task.request_cancellation()
                    }
                }
            })
        });

        for thread in threads {
            thread.join().unwrap();
        }
    });

    tasks
        .into_iter()
        .map(|x| x.join().map(|y| Arc::into_inner(y).unwrap().0))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parallel_execute() {
        //let tasks = (0..10).into_iter().map(|_i| todo!());
    }
}
