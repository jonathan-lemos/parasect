use std::collections::btree_map::CursorMut;
use std::collections::BTreeMap;
use std::ops::Bound::Included;
use ibig::{IBig, UBig};
use crate::collections::collect_vec::CollectVec;
use crate::range::numeric_range::{consolidate_range_stream, MaybeSplitNumericRange, NumericRange};
use crate::unwrap_or;
use MaybeSplitNumericRange::{NotSplit, Split};

/// A set of continuous ranges of integers.
/// This can represent any subset of [-inf, inf].
///
// Internally, none of the ranges should overlap, and there should be no empty ranges.
// The key of the internal map should equal the first element of the range.
#[derive(PartialEq, Eq, Debug)]
pub struct NumericRangeSet {
    range_starts: BTreeMap<IBig, NumericRange>,
    count: UBig,
}

impl NumericRangeSet {
    pub fn new() -> Self {
        Self { range_starts: BTreeMap::new(), count: UBig::from(0usize) }
    }

    /// Applies the given binary shrinking function to the current cursor position.
    ///
    /// The cursor is then placed
    ///     * 2+ new elements -> between the first new element and second new element
    ///     * 1 new element   -> between the first new element and the element after the processed gap.
    ///     * 0 new elements  -> between the two elements after the processed gap.
    fn apply_binary_shrinker<I, F>(cursor_mut: &mut CursorMut<IBig, NumericRange>, mut count_mut: &mut UBig, ap: F)
        where I: Iterator<Item=NumericRange>,
              F: FnOnce(&NumericRange, &NumericRange) -> I {
        let prev = unwrap_or!(cursor_mut.peek_prev(), return).1.clone();
        let next = unwrap_or!(cursor_mut.peek_next(), return).1.clone();

        let old = vec!(prev.clone(), next.clone());
        let new = consolidate_range_stream(ap(&prev, &next));

        if old == new {
            cursor_mut.next();
            return;
        }

        cursor_mut.remove_prev();
        cursor_mut.remove_next();

        *count_mut -= prev.len();
        *count_mut -= next.len();

        let union = &prev | &next;
        fn in_union(x: &NumericRange, union: &MaybeSplitNumericRange) -> bool {
            match &union {
                Split(a, b) => a.contains_range(x) || b.contains_range(x),
                NotSplit(a) => a.contains_range(x)
            }
        }

        for range in new.into_iter().rev() {
            if !in_union(&range, &union) {
                panic!("Operator applied to apply_shrinker was expected to shrink ranges, but {} is not within {} | {}.", range, prev, next);
            }

            let first = unwrap_or!(range.first(), continue);
            *count_mut += range.len();
            cursor_mut.insert_after(first, range).unwrap();
        }

        cursor_mut.next();
    }

    /// Shrinks the ranges intersecting [low, high] inclusive using the given binary function.
    ///
    /// The two arguments passed to the functions are neighboring ranges in the tree.
    /// The last range may not intersect [low, high], but the first one will always be.
    ///
    /// This function also recursively shrinks the new elements it adds, if they intersect [low, high].
    ///
    /// Panics if each new range is not within the union of the ranges given as arguments to the function.
    fn shrink_range_binary<I, F>(&mut self, low: &IBig, high: &IBig, mut ap: F)
        where I: Iterator<Item=NumericRange>,
              F: FnMut(&NumericRange, &NumericRange) -> I {
        let mut cursor = self.range_starts.upper_bound_mut(Included(low));

        loop {
            let prev_option = cursor.peek_prev().map(|x| x.1.clone());
            let next_option = cursor.peek_next().map(|x| x.1.clone());

            match (prev_option, next_option) {
                (None, _) => {
                    cursor.next();
                }
                (_, None) => return,
                (Some(a), Some(b)) => {
                    let (a_low, a_high) = unwrap_or!(a.as_tuple(), {
                        cursor.next();
                        continue;
                    });

                    if &a_low > high {
                        return;
                    }

                    if &a_high < low {
                        cursor.next();
                        continue;
                    }

                    Self::apply_binary_shrinker(&mut cursor, &mut self.count, &mut ap);
                }
            }
        }
    }

    /// Adds a range to the NumericRangeSet.
    pub fn add(&mut self, range: NumericRange) {
        let (low, high) = unwrap_or!(range.as_tuple(), return);

        let mut cursor = self.range_starts.upper_bound_mut(Included(&low));

        if let Some((k, r)) = cursor.peek_prev() {
            if k == &low {
                // if there is a range with the same key, remove it
                self.count -= r.len();
                cursor.remove_prev();
            }
        }

        self.count += range.len();
        cursor.insert_before(low.clone(), range).unwrap();
        self.shrink_range_binary(&(low - 1), &high, |prev, next| (prev | next).into_iter());
    }

    /// `true` if any range in the NumericRangeSet contains the given number.
    pub fn contains<N: Into<IBig>>(&self, n: N) -> bool {
        let n = n.into();
        self.contains_range(NumericRange::from_endpoints_inclusive(n.clone(), n.clone()))
    }

    /// `true` if any range in the NumericRangeSet includes each number in the given range.
    pub fn contains_range(&self, range: NumericRange) -> bool
    {
        let first = unwrap_or!(range.first(), return false);
        let cursor = self.range_starts.upper_bound(Included(&first));

        match cursor.peek_prev() {
            None => false,
            Some(r) => r.1.contains_range(&range)
        }
    }

    /// Iterates over all ranges in the NumericRangeSet that intersect [low, high] inclusive.
    pub fn iter_range_inclusive<'a, A: Into<IBig>, B: Into<IBig>>(&'a self, low: A, high: B) -> impl Iterator<Item=NumericRange> + 'a {
        self.range_starts.range((Included(&low.into()), Included(&high.into())))
            .map(|x| x.1.clone())
    }

    /// Returns the maximum value of any range in the NumericRangeSet.
    pub fn max(&self) -> Option<IBig> {
        self.range_starts.last_key_value().and_then(|x| x.1.last())
    }

    /// Returns the minimum value of any range in the NumericRangeSet.
    pub fn min(&self) -> Option<IBig> {
        self.range_starts.first_key_value().and_then(|x| x.1.first())
    }

    /// Removes the given range from all ranges in the NumericRangeSet.
    pub fn remove(&mut self, range: &NumericRange) {
        let (low, high) = unwrap_or!(range.as_tuple(), return);

        let mut cursor = self.range_starts.upper_bound_mut(Included(&low));

        loop {
            let prev = unwrap_or!(cursor.peek_prev()
                .map(|x| x.1.clone()), {
                    if let None = cursor.next() {
                        return;
                    } else {
                        continue;
                    }
                });

            let (prev_lo, prev_hi) = unwrap_or!(prev.as_tuple(), panic!("Range should not be empty."));

            if prev_hi < low {
                if let None = cursor.next() {
                    return;
                } else {
                    continue;
                }
            }

            if high < prev_lo {
                return;
            }

            self.count -= prev.len();
            cursor.remove_prev();

            let transformed_prev = consolidate_range_stream((&prev - range).into_iter());
            for range in transformed_prev.into_iter().rev() {
                let key = unwrap_or!(range.first(), continue);
                self.count += range.len();
                cursor.insert_after(key, range).unwrap();
            }

            cursor.next();
        }
    }

    /// Returns the difference between the highest element and the lowest element in the NumericRangeSet, if not empty.
    pub fn span(&self) -> Option<UBig> {
        self.min().and_then(|min|
            self.max().map(|max|
                UBig::try_from(max - min).unwrap()))
    }
}

mod tests {
    use super::*;

    fn empty() -> NumericRange {
        NumericRange::empty()
    }

    fn r<A: Into<IBig>, B: Into<IBig>>(low: A, high: B) -> NumericRange {
        NumericRange::from_endpoints_inclusive(low, high)
    }

    fn as_vec(s: &NumericRangeSet) -> Vec<(IBig, NumericRange)> {
        s.range_starts.iter()
            .map(|x| (x.0.clone(), x.1.clone()))
            .collect_vec()
    }

    fn kvs(a: &[(NumericRange)]) -> Vec<(IBig, NumericRange)> {
        a.into_iter()
            .map(|x| (x.first().unwrap(), x.clone()))
            .collect_vec()
    }

    fn assert_ranges_dont_overlap(s: &NumericRangeSet) {
        let range_vec = as_vec(s).into_iter()
            .map(|x| x.1)
            .collect_vec();

        for (a, b) in range_vec.iter().zip(range_vec.iter().skip(1)) {
            assert!(a.disjoint_to(b), "Ranges should not overlap, but {} and {} do.", a, b);
        }
    }

    fn assert_keys_equal_start_of_ranges(s: &NumericRangeSet) {
        for (k, v) in as_vec(s) {
            let first = unwrap_or!(v.first(),
                panic!("There should not be any empty ranges in the set, but there was one."));
            assert_eq!(first, k,
                       "All keys should be the first element of their range, but bad pair ({}, {}) was found.", k, v);
        }
    }

    fn assert_count_correct(s: &NumericRangeSet) {
        let mut actual = UBig::from(0usize);

        for (_, range) in as_vec(s) {
            actual += range.len();
        }

        assert_eq!(&actual, &s.count,
                   "Internal .count is incorrect. Count variable = {}, actual sum of ranges = {}", &s.count, &actual);
    }

    fn assert_invariants(s: &NumericRangeSet) {
        assert_ranges_dont_overlap(&s);
        assert_keys_equal_start_of_ranges(&s);
        assert_count_correct(&s);
    }

    #[test]
    fn test_add_basic() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 3));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 3)]));
    }

    #[test]
    fn test_add_before() {
        let mut s = NumericRangeSet::new();
        s.add(r(5, 9));
        s.add(r(1, 3));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 3), r(5, 9)]));
    }

    #[test]
    fn test_add_after() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 3));
        s.add(r(5, 9));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 3), r(5, 9)]));
    }

    #[test]
    fn test_add_overlapping_1() {
        let mut s = NumericRangeSet::new();
        s.add(r(3, 5));
        s.add(r(1, 9));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 9)]));
    }

    #[test]
    fn test_add_overlapping_2() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 5));
        s.add(r(3, 9));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 9)]));
    }

    #[test]
    fn test_add_overlapping_3() {
        let mut s = NumericRangeSet::new();
        s.add(r(3, 9));
        s.add(r(1, 5));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 9)]));
    }

    #[test]
    fn test_add_empty_noop() {
        let mut s = NumericRangeSet::new();
        s.add(empty());

        assert_invariants(&s);
        assert_eq!(as_vec(&s), Vec::new());
    }

    #[test]
    fn test_add_collision() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 5));
        s.add(r(1, 9));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 9)]));
    }

    #[quickcheck]
    fn qc_add_many(seq: Vec<(i16, i16)>) {
        let ranges = seq.into_iter().map(|(a, b)| {
            if a < b {
                r(a, b)
            } else {
                r(b, a)
            }
        }).collect_vec();

        let mut s = NumericRangeSet::new();

        for element in ranges.clone() {
            s.add(element);
            assert_invariants(&s);
        }

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&consolidate_range_stream(ranges.into_iter())));
    }

    #[test]
    fn test_remove_range_split() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 10));
        s.remove(&r(3, 9));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 2), r(10, 10)]));
    }
}