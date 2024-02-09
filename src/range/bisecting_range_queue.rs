use crate::range::numeric_range::NumericRange;
use crate::unwrap_or;
use crossbeam_channel::{unbounded, Receiver, Sender};
use dashmap::DashMap;
use ibig::IBig;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

struct RangeQueueNode {
    range: NumericRange,
    left: Option<Arc<RwLock<RangeQueueNode>>>,
    right: Option<Arc<RwLock<RangeQueueNode>>>,
    invalidated: bool,
}

impl RangeQueueNode {
    pub fn new(range: NumericRange) -> Self {
        Self {
            range,
            left: None,
            right: None,
            invalidated: false,
        }
    }
}

/// Produces a sequence of points that bisect the input space.
///
/// Ranges can also be "invalidated", preventing them from being selected in the future.
pub struct BisectingRangeQueue {
    range_sender: Sender<NumericRange>,
    range_receiver: Receiver<NumericRange>,
    nodes: DashMap<NumericRange, Arc<RwLock<RangeQueueNode>>>,
    queue_length: RwLock<usize>,
    on_invalidation: Option<Sender<NumericRange>>,
}

impl BisectingRangeQueue {
    /// Creates a new BisectingRangeQueue that bisects the given range.
    ///
    /// `on_invalidation` will receive messages whenever a range is invalidated.
    /// Sending a message to `on_invalidation` requires a read lock to a node of the given range,
    /// so if `on_invalidation` is bounded, its corresponding receiver should not block for `dequeue()`, as that can cause deadlocks.
    pub fn new(initial_range: NumericRange, on_invalidation: Option<Sender<NumericRange>>) -> Self {
        let (send, recv) = unbounded();

        let map = DashMap::new();

        if !initial_range.is_empty() {
            map.insert(
                initial_range.clone(),
                Arc::new(RwLock::new(RangeQueueNode::new(initial_range.clone()))),
            );

            send.send(initial_range.clone())
                .expect("channel should not be disconnected here");
        }

        Self {
            range_sender: send,
            range_receiver: recv,
            nodes: map,
            queue_length: RwLock::new(if initial_range.is_empty() { 0 } else { 1 }),
            on_invalidation,
        }
    }

    fn split(range: &NumericRange) -> (IBig, NumericRange, NumericRange) {
        let (low, high) = range.as_tuple().expect("should not split an empty range");

        let mid = (low + high) / 2;

        let left = range.truncate_end(&(&mid - 1));
        let right = range.truncate_start(&(&mid + 1));

        (mid, left, right)
    }

    fn pop_next_valid_node(&self) -> Option<(NumericRange, Arc<RwLock<RangeQueueNode>>)> {
        loop {
            {
                let length = self.queue_length.read().unwrap();

                if length.deref() == &0 {
                    return None;
                }
            }

            let range = match self.range_receiver.recv() {
                Ok(r) => r,
                // if this fails, the channel disconnected, so we want to return None
                Err(_) => return None,
            };

            {
                let mut count_mut = self.queue_length.write().unwrap();
                *count_mut -= 1;
            }

            let node = self
                .nodes
                .get(&range)
                .expect("cannot dequeue a range without inserting it into the map first")
                .clone();

            {
                let guard = node.read().unwrap();
                if guard.invalidated {
                    continue;
                }
            }

            return Some((range, node.clone()));
        }
    }

    fn append(&self, range: NumericRange, result: &mut Option<Arc<RwLock<RangeQueueNode>>>) {
        if range.is_empty() {
            return;
        }

        let entry = Arc::new(RwLock::new(RangeQueueNode::new(range.clone())));
        self.nodes.insert(range.clone(), entry.clone());
        *result = Some(entry);

        self.range_sender
            .send(range)
            .expect("channel should not be closed");

        let mut length_mut = self.queue_length.write().unwrap();
        *length_mut += 1;
    }

    /// Gets a split point along with the ranges to the left and right
    /// (neither including the split point), or None if there are no ranges left.
    ///
    /// Either or both ranges returned can be empty.
    pub fn dequeue(&self) -> Option<(IBig, NumericRange, NumericRange)> {
        let (range, node) = unwrap_or!(self.pop_next_valid_node(), return None);

        let (split_point, left, right) = Self::split(&range);

        let mut guard = node.write().unwrap();

        self.append(left.clone(), &mut guard.left);
        self.append(right.clone(), &mut guard.right);

        Some((split_point, left, right))
    }

    /// Marks a range (and all ranges within that range) as invalid, meaning they will not be
    /// present in subsequent dequeue() calls.
    ///
    /// Invalidating an empty range is a no-op.
    ///
    /// Panics if the given range was not produced by a previous call to this queue's `dequeue()`.
    pub fn invalidate(&self, range: &NumericRange) {
        if range.is_empty() {
            return;
        }

        let node = self
            .nodes
            .get(range)
            .expect("called invalidate() with a range not previously returned")
            .clone();

        let mut q = vec![node.clone()];

        while !q.is_empty() {
            let cur = q.pop().unwrap();

            {
                let mut write_guard = cur.write().unwrap();
                write_guard.invalidated = true;
            }

            let guard = cur.read().unwrap();

            if let Some(s) = &self.on_invalidation {
                // ignore if the channel is cancelled
                let _ = s.send(guard.range.clone());
            }

            if let Some(l) = &guard.left {
                q.push(l.clone());
            }

            if let Some(r) = &guard.right {
                q.push(r.clone());
            }
        }
    }

    /// Returns `true` if the given range was invalidated, `false` if not.
    ///
    /// An empty range is always invalid.
    ///
    /// Panics if the given range was not produced by a previous call to this queue's `dequeue()`.
    pub fn invalidated(&self, range: &NumericRange) -> bool {
        if range.is_empty() {
            return true;
        }

        let node = self
            .nodes
            .get(range)
            .expect("called invalidated() with a range not previously returned")
            .clone();

        let guard = node.read().unwrap();
        guard.invalidated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::collect_collection::CollectHashSet;
    use crate::test_util::test_util::test_util::{ib, r};
    use crossbeam_channel::bounded;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::thread;

    #[test]
    fn test_dequeue_produces_all_elements() {
        let mut ns = HashSet::new();
        let q = BisectingRangeQueue::new(r(1, 10), None);

        while let Some((pt, _, _)) = q.dequeue() {
            ns.insert(pt);
        }

        assert_eq!(ns, r(1, 10).iter().collect_hashset());
    }

    #[test]
    fn test_dequeue_check_first() {
        let q = BisectingRangeQueue::new(r(0, 10), None);

        assert_eq!(q.dequeue(), Some((ib(5), r(0, 4), r(6, 10))));
    }

    #[test]
    fn test_dequeue_invalidate_first_only_hits_others() {
        let mut ns = HashSet::new();
        let q = BisectingRangeQueue::new(r(0, 10), None);

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
        let q = BisectingRangeQueue::new(r(0, 10), None);

        let (pt, a, _b) = q.dequeue().unwrap();
        let (pt2, a2, _b2) = q.dequeue().unwrap();

        ns.insert(pt.clone());
        ns.insert(pt2.clone());

        q.invalidate(&a);

        while let Some((pt, _, _)) = q.dequeue() {
            ns.insert(pt);
        }

        assert!(q.invalidated(&a));
        assert!(q.invalidated(&a2));
        assert_eq!(
            ns,
            [pt, pt2, ib(6), ib(7), ib(8), ib(9), ib(10)]
                .into_iter()
                .map(ib)
                .collect_hashset()
        );
    }

    #[test]
    fn test_invalidation_alert() {
        let (send, recv) = bounded(2);

        let q = BisectingRangeQueue::new(r(0, 10), Some(send));

        let (_, a, _) = q.dequeue().unwrap();
        q.invalidate(&a);

        let (_, a2, _) = q.dequeue().unwrap();
        q.invalidate(&a2);

        assert_eq!(a, recv.recv().unwrap());
        assert_eq!(a2, recv.recv().unwrap());
    }

    proptest! {
        #[test]
        fn test_binary_search(a in 1..100, b in 1..100) {
            let (send, recv) = unbounded();

            prop_assume!(a <= b);

            let a = IBig::from(a);
            let b = IBig::from(b);

            let q = BisectingRangeQueue::new(r(0, b.clone()), Some(send));

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

            let mut rejected = Vec::new();
            while let Ok(range) = recv.try_recv() {
                rejected.push(range);
            }

            assert_eq!(res, Some(a.clone()));
            assert!(rejected.into_iter().all(|r| !r.contains(a.clone())));
        }

        #[test]
        fn test_binary_search_async(a in 1..1000, b in 1..1000) {
            let (send, recv) = unbounded();

            prop_assume!(a <= b);

            let a = IBig::from(a);
            let b = IBig::from(b);

            let q = BisectingRangeQueue::new(r(0, b.clone()), Some(send));

            thread::scope(|scope| {
                let qref = &q;
                let aref = &a;
                while let Some((point, left, right)) = qref.dequeue() {
                    scope.spawn(move || {
                        if &point < aref {
                            qref.invalidate(&left);
                            assert!(qref.invalidated(&left));
                        } else if &point > aref {
                            qref.invalidate(&right);
                            assert!(qref.invalidated(&right));
                        }
                    });
                }
            });

            let mut rejected = Vec::new();
            while let Ok(range) = recv.try_recv() {
                rejected.push(range);
            }

            assert!(rejected.into_iter().all(|r| !r.contains(a.clone())));
        }
    }
}
