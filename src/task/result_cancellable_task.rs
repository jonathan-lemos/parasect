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
    use crate::task::test_util::*;
    use proptest::prelude::*;

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

    #[test]
    fn test_ct_invariants_ok() {
        assert_cancellabletask_invariants(|| {
            ResultCancellableTask::new(wrap_result(FreeCancellableTask::new(69)))
        })
    }

    #[test]
    fn test_ct_invariants_err() {
        assert_cancellabletask_invariants(|| ResultCancellableTask::new(wrap_err(69)))
    }

    proptest! {
        #[test]
        fn test_thread_safe_ok(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| ResultCancellableTask::new(
                wrap_result(FreeCancellableTask::new(i))
            ));
        }

        #[test]
        fn test_thread_safe_err(i in 1..10000) {
            assert_cancellabletask_thread_safe(|| ResultCancellableTask::new(
                wrap_err(i)
            ));
        }
    }
}
