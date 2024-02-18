#[cfg(test)]
pub mod test_util {
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

    pub(crate) use assert_result_eq;
}
