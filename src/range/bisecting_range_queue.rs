use crate::range::numeric_range::NumericRange;
use crate::range::numeric_range_set::NumericRangeSet;
use crate::unwrap_or;
use ibig::IBig;
use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};

/// Produces a sequence of points that bisect the input space.
///
/// Ranges can also be "invalidated", preventing them from being selected in the future.
pub struct BisectingRangeQueue {
    // mutex because all operations on this mutate it
    range_queue: Mutex<VecDeque<NumericRange>>,
    // rwlock because reads can happen independently of writes
    invalid: RwLock<NumericRangeSet>,
}

impl BisectingRangeQueue {
    /// Creates a new BisectingRangeQueue that bisects the given range.
    pub fn new(initial_range: NumericRange) -> Self {
        let mut q = VecDeque::new();
        q.push_back(initial_range.clone());

        Self {
            range_queue: Mutex::new(q),
            invalid: RwLock::new(NumericRangeSet::new()),
        }
    }

    fn split(range: &NumericRange) -> (IBig, NumericRange, NumericRange) {
        let (low, high) = range.as_tuple().expect("should not split an empty range");

        let mid = (low + high) / 2;

        let left = range.truncate_end(&(&mid - 1));
        let right = range.truncate_start(&(&mid + 1));

        (mid, left, right)
    }

    fn pop_next_valid_node(&self) -> Option<NumericRange> {
        let mut range_queue = self.range_queue.lock().unwrap();

        loop {
            let range = unwrap_or!(range_queue.pop_front(), return None);

            let invalid = self.invalid.read().unwrap();
            if invalid.contains_range(&range) {
                continue;
            }

            return Some(range);
        }
    }

    fn append(&self, range: NumericRange) {
        if range.is_empty() || self.invalid.read().unwrap().contains_range(&range) {
            return;
        }

        let mut range_guard = self.range_queue.lock().unwrap();
        range_guard.push_back(range.clone());
    }

    /// Gets a split point along with the ranges to the left and right
    /// (neither including the split point), or None if there are no ranges left.
    ///
    /// Either or both ranges returned can be empty.
    pub fn dequeue(&self) -> Option<(IBig, NumericRange, NumericRange)> {
        let range = unwrap_or!(self.pop_next_valid_node(), return None);

        let (split_point, left, right) = Self::split(&range);

        self.append(left.clone());
        self.append(right.clone());

        Some((split_point, left, right))
    }

    /// Marks a range (and all ranges within that range) as invalid, meaning they will not be
    /// present in subsequent dequeue() calls.
    ///
    /// Invalidating an empty range is a no-op.
    pub fn invalidate(&self, range: &NumericRange) {
        self.invalid.write().unwrap().add(range.clone());
    }

    /// Returns `true` if the given range was invalidated, `false` if not.
    ///
    /// An empty range is always invalid.
    pub fn range_invalidated(&self, range: &NumericRange) -> bool {
        self.invalid.read().unwrap().contains_range(range)
    }

    /// Returns `true` if the given range was invalidated, `false` if not.
    ///
    /// An empty range is always invalid.
    pub fn point_invalidated<P: Into<IBig>>(&self, point: P) -> bool {
        self.invalid.read().unwrap().contains(point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::collect_collection::CollectHashSet;
    use crate::test_util::test_util::test_util::{ib, r};
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::thread;

    #[test]
    fn test_dequeue_produces_all_elements() {
        let mut ns = HashSet::new();
        let q = BisectingRangeQueue::new(r(1, 10));

        while let Some((pt, _, _)) = q.dequeue() {
            ns.insert(pt);
        }

        assert_eq!(ns, r(1, 10).iter().collect_hashset());
    }

    #[test]
    fn test_dequeue_check_first() {
        let q = BisectingRangeQueue::new(r(0, 10));

        assert_eq!(q.dequeue(), Some((ib(5), r(0, 4), r(6, 10))));
    }

    #[test]
    fn test_dequeue_invalidate_first_only_hits_others() {
        let mut ns = HashSet::new();
        let q = BisectingRangeQueue::new(r(0, 10));

        q.dequeue();

        q.invalidate(&r(6, 10));

        while let Some((pt, _, _)) = q.dequeue() {
            ns.insert(pt);
        }

        assert_eq!(ns, r(0, 4).iter().collect_hashset());
    }

    #[test]
    fn test_dequeue_invalidate_recurses() {
        let mut ns = HashSet::new();
        let q = BisectingRangeQueue::new(r(0, 10));

        let (pt, a, _b) = q.dequeue().unwrap();
        let (pt2, a2, _b2) = q.dequeue().unwrap();

        ns.insert(pt.clone());
        ns.insert(pt2.clone());

        q.invalidate(&a);

        while let Some((pt, _, _)) = q.dequeue() {
            ns.insert(pt);
        }

        assert!(q.range_invalidated(&a));
        assert!(q.range_invalidated(&a2));
        assert_eq!(
            ns,
            [pt, pt2, ib(6), ib(7), ib(8), ib(9), ib(10)]
                .into_iter()
                .map(ib)
                .collect_hashset()
        );
    }

    proptest! {
        #[test]
        fn test_binary_search(a in 1..100, b in 1..100) {
            prop_assume!(a <= b);

            let a = IBig::from(a);
            let b = IBig::from(b);

            let q = BisectingRangeQueue::new(r(0, b.clone()));

            let mut res = None;

            while let Some((point, left, right)) = q.dequeue() {
                if point < a {
                    q.invalidate(&left);
                } else if point > a {
                    q.invalidate(&right);
                } else {
                    res = Some(point);
                    break;
                }
            }

            assert_eq!(res, Some(a.clone()));
        }

        #[test]
        fn test_binary_search_async_no_deadlock(a in 1..1000, b in 1..1000) {
            prop_assume!(a <= b);

            let a = IBig::from(a);
            let b = IBig::from(b);

            let q = BisectingRangeQueue::new(r(0, b.clone()));

            thread::scope(|scope| {
                let qref = &q;
                let aref = &a;
                while let Some((point, left, right)) = qref.dequeue() {
                    scope.spawn(move || {
                        if &point < aref {
                            qref.invalidate(&left);
                            assert!(qref.range_invalidated(&left));
                        } else if &point > aref {
                            qref.invalidate(&right);
                            assert!(qref.range_invalidated(&right));
                        }
                    });
                }
            });
        }
    }
}
