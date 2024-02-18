use std::cell::UnsafeCell;
use std::sync::RwLock;

/// A cell holding a value that can be taken only once.
pub struct SingleUseCell<T> {
    inner: RwLock<UnsafeCell<Option<T>>>,
}

impl<T> SingleUseCell<T> {
    /// Makes a new `SingleUseCell` holding the given value.
    pub fn new(value: T) -> Self {
        Self {
            inner: RwLock::new(UnsafeCell::new(Some(value))),
        }
    }

    /// Makes a new `SingleUseCell` holding no value.
    pub fn empty() -> Self {
        Self {
            inner: RwLock::new(UnsafeCell::new(None)),
        }
    }

    /// Returns `true` if and only if a value is present in the `SingleUseCell`.
    pub fn has_value(&self) -> bool {
        let inner_read = self.inner.read().unwrap();

        // we need to get an immutable reference to the cell's contents without a write lock
        // safe because cell mutation cannot happen without a write lock
        unsafe { inner_read.get().as_ref().unwrap().is_some() }
    }

    /// Removes the value from the `SingleUseValue` and returns it, if present, or returns None if no value is present.
    pub fn take(&self) -> Option<T> {
        if !self.has_value() {
            return None;
        }

        let mut inner_write = self.inner.write().unwrap();
        let mut_ref = inner_write.get_mut();

        mut_ref.take()
    }
}

// Should be thread-safe as long as T can be sent across threads.
// This is because this class is immutable except for the part that necessitates a write lock.
// All other accesses require a read lock.
unsafe impl<T: Send> Send for SingleUseCell<T> {}
unsafe impl<T: Send> Sync for SingleUseCell<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::thread;

    #[test]
    fn test_take() {
        let c = SingleUseCell::new(69);

        assert_eq!(c.take(), Some(69));
    }

    #[test]
    fn test_take_twice() {
        let c = SingleUseCell::new(69);

        assert_eq!(c.take(), Some(69));
        assert_eq!(c.take(), None);
    }

    #[test]
    fn test_has_value() {
        let c = SingleUseCell::new(69);

        assert!(c.has_value());
        c.take();
        assert!(!c.has_value());
    }

    #[test]
    fn test_empty() {
        let c = SingleUseCell::<()>::empty();

        assert!(!c.has_value());
        assert_eq!(c.take(), None);
    }

    proptest! {
        #[test]
        fn fuzz_take_threadsafe(i in 1..10000) {
            let c = SingleUseCell::new(i);

            let (v1, v2) = thread::scope(|scope| {
                let h1 = scope.spawn(|| c.take());
                let h2 = scope.spawn(|| c.take());

                (h1.join().unwrap(), h2.join().unwrap())
            });

            assert!(v1 == Some(i) && v2 == None || v2 == Some(i) && v1 == None);
        }

        #[test]
        fn fuzz_take_hasvalue_threadsafe(i in 1..10000) {
            let c = SingleUseCell::new(i);

            let (v1, v2) = thread::scope(|scope| {
                scope.spawn(|| c.has_value());
                let h1 = scope.spawn(|| c.take());
                scope.spawn(|| c.has_value());
                let h2 = scope.spawn(|| c.take());
                scope.spawn(|| c.has_value());

                (h1.join().unwrap(), h2.join().unwrap())
            });

            assert!(v1 == Some(i) && v2 == None || v2 == Some(i) && v1 == None);
        }
    }
}
