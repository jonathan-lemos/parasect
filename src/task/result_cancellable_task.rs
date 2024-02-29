use crate::messaging::mailbox::Mailbox;
use crate::task::cancellable_task::CancellableTask;
use crate::task::map_cancellable_task::MapValueCancellableTask;
use crate::threading::async_value::AsyncValue;

pub struct ResultCancellableTask<T, E, C>
where
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    inner_task: Option<MapValueCancellableTask<T, Result<T, E>, C>>,
    value: AsyncValue<Option<Result<T, E>>>,
}

impl<T, E, C> ResultCancellableTask<T, E, C>
where
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    pub fn new(result: Result<C, E>) -> Self {
        match result {
            Ok(t) => {
                let value = AsyncValue::new();
                let t = t.map(Ok);
                t.notify_when_done(value.clone());
                Self {
                    inner_task: Some(t),
                    value,
                }
            }
            Err(e) => Self {
                inner_task: None,
                value: AsyncValue::from(Some(Err(e))),
            },
        }
    }
}

impl<T, E, C> CancellableTask<Result<T, E>> for ResultCancellableTask<T, E, C>
where
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
    C: CancellableTask<T>,
{
    fn notify_when_done(
        &self,
        notifiable: impl Mailbox<'static, Message = Option<Result<T, E>>> + 'static,
    ) {
        self.value.notify_when_done(notifiable)
    }

    fn request_cancellation(&self) -> () {
        self.value.send_msg(None);
        if let Some(s) = &self.inner_task {
            s.request_cancellation();
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
    fn test_wait_ok() {
        let r = ResultCancellableTask::new(wrap_result(FreeCancellableTask::new(69)));

        let x = r.wait();
        assert_eq!(x, Some(Ok(69)));
    }

    #[test]
    fn test_wait_err() {
        let r = ResultCancellableTask::new(wrap_err(69));

        let x = r.wait();
        assert_eq!(x, Some(Err(69)));
    }

    #[test]
    fn test_cancel_ok() {
        let r = ResultCancellableTask::new(wrap_result(FreeCancellableTask::new(69)));

        r.request_cancellation();

        let x = r.wait();
        assert_eq!(x, None);
    }

    #[test]
    fn test_cancel_err() {
        let r = ResultCancellableTask::new(wrap_err(69));

        r.request_cancellation();

        let x = r.wait();
        assert_eq!(x, Some(Err(69)));
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
