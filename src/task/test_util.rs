#[cfg(test)]
pub mod test_util {
    use std::fmt::Debug;
    use std::sync::Arc;
    use std::thread;
    use std::thread::Scope;
    use std::time::Duration;
    use crate::task::cancellable_message::CancellableMessage;
    use crate::task::cancellable_task::CancellableTask;

    pub trait ResultLike<T> {
        fn to_result(self) -> Option<Arc<T>>;
    }

    impl<T> ResultLike<T> for Option<Arc<T>> {
        fn to_result(self) -> Option<Arc<T>> {
            self
        }
    }

    impl<T> ResultLike<T> for Option<T> {
        fn to_result(self) -> Option<Arc<T>> {
            self.map(Arc::new)
        }
    }

    impl<T> ResultLike<T> for T {
        fn to_result(self) -> Option<Arc<T>> {
            Some(Arc::new(self))
        }
    }

    pub fn assert_result_eq<T: Debug + PartialEq, A: ResultLike<T>, B: ResultLike<T>>(a: A, b: B) {
        assert_eq!(a.to_result(), b.to_result());
    }

    pub fn test_cancel_before<T: Send + Sync, C: CancellableTask<T>, F: FnOnce(&C) -> ()>(cancellable_task: C, setup: F) -> Option<Arc<T>> {
        setup(&cancellable_task);
        cancellable_task.request_cancellation();
        cancellable_task.join()
    }

    pub fn test_cancel_after<T: Send + Sync, C: CancellableTask<T>, F: FnOnce(&C) -> ()>(cancellable_task: C, setup: F) -> Option<Arc<T>> {
        thread::scope(|scope| {
            let t = scope.spawn(|| cancellable_task.join());

            thread::sleep(Duration::from_millis(5));

            setup(&cancellable_task);
            cancellable_task.request_cancellation();

            t.join().unwrap()
        })
    }

    pub fn test_flakefind_join_before<T: Send + Sync, C: CancellableTask<T>, F: Fn(&C) -> () + Send + Sync>(cancellable_task: &C, send: F) -> Vec<Option<Arc<T>>> {
        thread::scope(|scope| {
            let handles = (0..100).into_iter().map(|i| {
                scope.spawn(|| send(&cancellable_task));
                scope.spawn(|| cancellable_task.join())
            });

            handles.map(|h| h.join().unwrap()).collect()
        })
    }

    pub fn test_flakefind_join_after<T: Send + Sync, C: CancellableTask<T>, F: Fn(&C) -> () + Send + Sync>(cancellable_task: &C, send: F) -> Vec<Option<Arc<T>>> {
        thread::scope(|scope| {
            let handles = (0..100).into_iter().map(|i| {
                let r = scope.spawn(|| cancellable_task.join());
                scope.spawn(|| send(&cancellable_task));
                r
            });

            handles.map(|h| h.join().unwrap()).collect()
        })
    }

    pub fn test_flakefind_join<T: Send + Sync, C: CancellableTask<T>, F: Fn(&C) -> () + Send + Sync + Clone>(cancellable_task: &C, send: F) -> Vec<Option<Arc<T>>> {
        let mut v = test_flakefind_join_before(cancellable_task, send.clone());
        v.append(&mut test_flakefind_join_after(cancellable_task, send.clone()));
        v
    }
}