use crate::task::cancellable_task::CancellableTask;
use crate::threading::single_use_cell::SingleUseCell;
use std::sync::OnceLock;

pub struct ResultCancellableTask<T, E, C>
where
    T: Send + Sync,
    E: Send + Sync,
    C: CancellableTask<T>,
{
    inner_task: SingleUseCell<Option<C>>,
    value: OnceLock<Option<Result<T, E>>>,
}

impl<T, E, C> ResultCancellableTask<T, E, C>
where
    T: Send + Sync,
    E: Send + Sync,
    C: CancellableTask<T>,
{
    pub fn new(result: Result<C, E>) -> Self {
        match result {
            Ok(t) => Self {
                inner_task: SingleUseCell::new(Some(t)),
                value: OnceLock::new(),
            },
            Err(e) => Self {
                inner_task: SingleUseCell::empty(),
                value: OnceLock::from(Some(Err(e))),
            },
        }
    }
}

impl<T, E, C> CancellableTask<Result<T, E>> for ResultCancellableTask<T, E, C>
where
    T: Send + Sync,
    E: Send + Sync,
    C: CancellableTask<T>,
{
    fn join(&self) -> Option<&Result<T, E>> {
        self.value
            .get_or_init(|| self.inner_task.take().unwrap().unwrap().join_into().map(Ok))
            .as_ref()
    }

    fn join_into(mut self) -> Option<Result<T, E>> {
        self.join();
        self.value.take().unwrap()
    }

    fn request_cancellation(&self) -> () {
        let _ = self.value.set(None);
        match self.inner_task.take() {
            Some(Some(c)) => c.request_cancellation(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::free_cancellable_task::FreeCancellableTask;
    use proptest::prelude::*;
    use std::thread;

    fn wrap_result<T>(t: T) -> Result<T, ()> {
        Ok(t)
    }

    fn wrap_err<E>(e: E) -> Result<FreeCancellableTask<i32>, E> {
        Err(e)
    }

    #[test]
    fn test_join_ok() {
        let r = ResultCancellableTask::new(wrap_result(FreeCancellableTask::new(69)));

        let x = r.join();
        assert_eq!(x, Some(&Ok(69)))
    }

    #[test]
    fn test_join_err() {
        let r = ResultCancellableTask::new(wrap_err(69));

        let x = r.join();
        assert_eq!(x, Some(&Err(69)))
    }

    #[test]
    fn test_cancel_ok() {
        let r = ResultCancellableTask::new(wrap_result(FreeCancellableTask::new(69)));

        r.request_cancellation();

        let x = r.join();
        assert_eq!(x, None)
    }

    #[test]
    fn test_cancel_err() {
        let r = ResultCancellableTask::new(wrap_err(69));

        r.request_cancellation();

        let x = r.join();
        assert_eq!(x, Some(&Err(69)))
    }

    proptest! {
        #[test]
        fn fuzz_join_cancel_no_panic(i in 1..10000) {
            let r = ResultCancellableTask::new(wrap_result(FreeCancellableTask::new(i)));

            thread::scope(|scope| {
                scope.spawn(|| r.join());
                scope.spawn(|| r.request_cancellation());
            });
        }

        #[test]
        fn fuzz_join_idempotent(i in 1..10000) {
            let r = ResultCancellableTask::new(wrap_result(FreeCancellableTask::new(i)));

            let (v1, v2) = thread::scope(|scope| {
                let t1 = scope.spawn(|| r.join());
                let t2 = scope.spawn(|| r.join());

                (t1.join().unwrap(), t2.join().unwrap())
            });

            assert_eq!(v1, Some(&Ok(i)));
            assert_eq!(v2, Some(&Ok(i)));
        }
    }
}
