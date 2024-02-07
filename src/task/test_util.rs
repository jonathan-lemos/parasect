#[cfg(test)]
pub mod test_util {
    use std::sync::Arc;

    pub trait ResultLike<T>: Clone {
        fn to_result(&self) -> Option<Arc<T>>;
    }

    impl<T: Clone> ResultLike<T> for Option<Arc<T>> {
        fn to_result(&self) -> Option<Arc<T>> {
            self.clone()
        }
    }

    impl<T: Clone> ResultLike<T> for Option<T> {
        fn to_result(&self) -> Option<Arc<T>> {
            self.as_ref().map(|x| Arc::new(x.clone()))
        }
    }

    impl<T: Clone> ResultLike<T> for T {
        fn to_result(&self) -> Option<Arc<T>> {
            Some(Arc::new(self.clone()))
        }
    }

    #[macro_export]
    macro_rules! assert_result_eq {
        ($a:expr, $b:expr) => {
            assert_eq!($a.to_result(), $b.to_result());
        };
    }
}
