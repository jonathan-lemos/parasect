use crate::collections::collect_collection::CollectVec;
use crate::range::numeric_range::{consolidate_range_stream, MaybeSplitNumericRange, NumericRange};
use crate::unwrap_or;
use ibig::{IBig, UBig};
use std::collections::btree_map::{Cursor, CursorMut};
use std::collections::BTreeMap;
use std::ops::Bound::Included;
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

#[allow(unused)]
impl NumericRangeSet {
    pub fn new() -> Self {
        Self {
            range_starts: BTreeMap::new(),
            count: UBig::from(0usize),
        }
    }

    /// Applies the given binary shrinking function to the current cursor position.
    ///
    /// The cursor is then placed
    ///     * 2+ new elements -> between the first new element and second new element
    ///     * 1 new element   -> between the first new element and the element after the processed gap.
    ///     * 0 new elements  -> between the two elements after the processed gap.
    fn apply_binary_shrinker<I, F>(
        cursor_mut: &mut CursorMut<IBig, NumericRange>,
        count_mut: &mut UBig,
        ap: F,
    ) where
        I: Iterator<Item = NumericRange>,
        F: FnOnce(&NumericRange, &NumericRange) -> I,
    {
        let prev = unwrap_or!(cursor_mut.peek_prev(), return).1.clone();
        let next = unwrap_or!(cursor_mut.peek_next(), return).1.clone();

        let old = vec![prev.clone(), next.clone()];
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
                NotSplit(a) => a.contains_range(x),
            }
        }

        for range in new.into_iter().rev() {
            if !in_union(&range, &union) {
                panic!("Operator applied to apply_shrinker was expected to shrink ranges, but {} is not within {} | {}.", range, prev, next);
            }

            let first = range.first().unwrap();
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
    where
        I: Iterator<Item = NumericRange>,
        F: FnMut(&NumericRange, &NumericRange) -> I,
    {
        let mut cursor = self.range_starts.upper_bound_mut(Included(low));

        loop {
            let prev_option = cursor.peek_prev().map(|x| x.1.clone());
            let next_option = cursor.peek_next().map(|x| x.1.clone());

            match (prev_option, next_option) {
                (None, _) => {
                    cursor.next();
                }
                (_, None) => return,
                (Some(a), Some(_)) => {
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
                // if there is a range with the same key
                if r.last().unwrap() >= high {
                    // return if that one is bigger or equal
                    return;
                }
                // otherwise remove the existing range
                self.count -= r.len();
                cursor.remove_prev();
            }
        }

        self.count += range.len();
        cursor.insert_before(low.clone(), range).unwrap();
        self.shrink_range_binary(&(low - 1), &high, |prev, next| (prev | next).into_iter());
    }

    /// `(min, max)`, if there's at least one value in the NumericRangeSet.
    pub fn bounds(&self) -> NumericRange {
        self.min()
            .and_then(|lo| {
                self.max()
                    .map(|hi| NumericRange::from_endpoints_inclusive(lo, hi))
            })
            .unwrap_or(NumericRange::empty())
    }

    /// `true` if any range in the NumericRangeSet contains the given number.
    pub fn contains<N: Into<IBig>>(&self, n: N) -> bool {
        let n = n.into();
        self.contains_range(&NumericRange::from_endpoints_inclusive(
            n.clone(),
            n.clone(),
        ))
    }

    /// `true` if any range in the NumericRangeSet includes each number in the given range.
    pub fn contains_range(&self, range: &NumericRange) -> bool {
        if range.is_empty() {
            return true;
        }

        let first = unwrap_or!(range.first(), return false);
        let cursor = self.range_starts.upper_bound(Included(&first));

        match cursor.peek_prev() {
            None => false,
            Some(r) => r.1.contains_range(&range),
        }
    }

    /// Iterates over all ranges in the NumericRangeSet.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = NumericRange> + 'a {
        self.range_starts.iter().map(|x| x.1.clone())
    }

    /// Iterates over all ranges in the NumericRangeSet that intersect [low, high] inclusive.
    pub fn iter_range<'a>(
        &'a self,
        range: &NumericRange,
    ) -> impl Iterator<Item = NumericRange> + 'a {
        let low = range.first().unwrap_or(IBig::from(0));

        NumericRangeSetIterator {
            cursor: self.range_starts.upper_bound(Included(&low)),
            range: range.clone(),
            end: false,
        }
    }

    /// Returns `true` if and only if the NumericRangeSet contains at least one element from the range or the range is empty.
    pub fn intersects_range(&self, range: &NumericRange) -> bool {
        return range.is_empty() || self.iter_range(&range).any(|_| true);
    }

    /// Returns the maximum value of any range in the NumericRangeSet.
    pub fn max(&self) -> Option<IBig> {
        self.range_starts.last_key_value().and_then(|x| x.1.last())
    }

    /// Returns the minimum value of any range in the NumericRangeSet.
    pub fn min(&self) -> Option<IBig> {
        self.range_starts
            .first_key_value()
            .and_then(|x| x.1.first())
    }

    /// Removes the given range from all ranges in the NumericRangeSet.
    pub fn remove(&mut self, range: &NumericRange) {
        let (low, high) = unwrap_or!(range.as_tuple(), return);

        let mut cursor = self.range_starts.upper_bound_mut(Included(&low));

        loop {
            let prev = unwrap_or!(cursor.peek_prev().map(|x| x.1.clone()), {
                if let None = cursor.next() {
                    return;
                } else {
                    continue;
                }
            });

            let (prev_lo, prev_hi) =
                unwrap_or!(prev.as_tuple(), panic!("Range should not be empty."));

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
        self.min()
            .and_then(|min| self.max().map(|max| UBig::try_from(max - min).unwrap()))
    }
}

pub struct NumericRangeSetIterator<'a> {
    cursor: Cursor<'a, IBig, NumericRange>,
    range: NumericRange,
    end: bool,
}

impl<'a> Iterator for NumericRangeSetIterator<'a> {
    type Item = NumericRange;

    fn next(&mut self) -> Option<Self::Item> {
        let (low, high) = unwrap_or!(self.range.as_tuple(), return None);

        if self.end {
            return None;
        }

        match self.cursor.peek_prev() {
            None => {
                if self.cursor.peek_next().is_none() {
                    None
                } else {
                    self.end = self.cursor.next().is_none();
                    self.next()
                }
            }
            Some(r) => {
                if r.1.last().unwrap() < low {
                    self.end = self.cursor.next().is_none();
                    self.next()
                } else if r.1.first().unwrap() > high {
                    None
                } else {
                    self.end = self.cursor.next().is_none();
                    Some(r.1.clone())
                }
            }
        }
    }
}

impl FromIterator<NumericRange> for NumericRangeSet {
    fn from_iter<T: IntoIterator<Item = NumericRange>>(iter: T) -> Self {
        let mut ret = NumericRangeSet::new();

        for range in iter {
            ret.add(range);
        }

        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::collect_collection::CollectHashSet;
    use crate::test_util::test_util::test_util::{ib, ub};
    use proptest::collection::vec;
    use proptest::prelude::*;
    use std::collections::HashSet;

    fn empty() -> NumericRange {
        NumericRange::empty()
    }

    fn r<A: Into<IBig>, B: Into<IBig>>(low: A, high: B) -> NumericRange {
        NumericRange::from_endpoints_inclusive(low, high)
    }

    fn as_vec(s: &NumericRangeSet) -> Vec<(IBig, NumericRange)> {
        s.range_starts
            .iter()
            .map(|x| (x.0.clone(), x.1.clone()))
            .collect_vec()
    }

    fn kvs(a: &[NumericRange]) -> Vec<(IBig, NumericRange)> {
        a.into_iter()
            .map(|x| (x.first().unwrap(), x.clone()))
            .collect_vec()
    }

    fn assert_ranges_dont_overlap(s: &NumericRangeSet) {
        let range_vec = as_vec(s).into_iter().map(|x| x.1).collect_vec();

        for (a, b) in range_vec.iter().zip(range_vec.iter().skip(1)) {
            assert!(
                a.disjoint_to(b),
                "Ranges should not overlap, but {} and {} do.",
                a,
                b
            );
        }
    }

    fn assert_keys_equal_start_of_ranges(s: &NumericRangeSet) {
        for (k, v) in as_vec(s) {
            let first = unwrap_or!(
                v.first(),
                panic!("There should not be any empty ranges in the set, but there was one.")
            );
            assert_eq!(first, k,
                       "All keys should be the first element of their range, but bad pair ({}, {}) was found.", k, v);
        }
    }

    fn assert_count_correct(s: &NumericRangeSet) {
        let mut actual = UBig::from(0usize);

        for (_, range) in as_vec(s) {
            actual += range.len();
        }

        assert_eq!(
            &actual, &s.count,
            "Internal .count is incorrect. Count variable = {}, actual sum of ranges = {}",
            &s.count, &actual
        );
    }

    fn assert_invariants(s: &NumericRangeSet) {
        assert_ranges_dont_overlap(&s);
        assert_keys_equal_start_of_ranges(&s);
        assert_count_correct(&s);
    }

    fn test_add_sequence(sequence: &[NumericRange]) {
        let mut s = NumericRangeSet::new();

        for elem in sequence {
            s.add(elem.clone());
            assert_invariants(&s);
        }

        assert_invariants(&s);
        assert_eq!(
            as_vec(&s),
            kvs(&consolidate_range_stream(
                sequence.into_iter().map(|x| x.clone())
            ))
        );
    }

    #[test]
    fn test_add_basic() {
        test_add_sequence(&[r(1, 3)]);
    }

    #[test]
    fn test_add_before() {
        test_add_sequence(&[r(1, 3), r(5, 9)])
    }

    #[test]
    fn test_add_after() {
        test_add_sequence(&[r(5, 9), r(1, 3)])
    }

    #[test]
    fn test_add_surrounding_1() {
        test_add_sequence(&[r(3, 5), r(1, 9)]);
    }

    #[test]
    fn test_add_surrounding_2() {
        test_add_sequence(&[r(1, 9), r(3, 5)]);
    }

    #[test]
    fn test_add_overlapping_1() {
        test_add_sequence(&[r(1, 5), r(3, 9)]);
    }

    #[test]
    fn test_add_overlapping_2() {
        test_add_sequence(&[r(3, 9), r(1, 5)]);
    }

    #[test]
    fn test_add_empty_noop() {
        test_add_sequence(&[empty()]);
    }

    #[test]
    fn test_add_collision_1() {
        test_add_sequence(&[r(1, 5), r(1, 9)]);
    }

    #[test]
    fn test_add_collision_2() {
        test_add_sequence(&[r(1, 9), r(1, 5)]);
    }

    #[test]
    fn test_remove_range_split() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 10));
        s.remove(&r(3, 9));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 2), r(10, 10)]));
    }

    #[test]
    fn test_remove_range_split_2() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 10));
        s.add(r(15, 20));
        s.remove(&r(5, 18));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(1, 4), r(19, 20)]));
    }

    #[test]
    fn test_remove_range_split_3() {
        let mut s = NumericRangeSet::new();
        s.add(r(-6, -3));
        s.add(r(1, 10));
        s.add(r(15, 20));
        s.add(r(25, 30));
        s.remove(&r(5, 18));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(-6, -3), r(1, 4), r(19, 20), r(25, 30)]));
    }

    #[test]
    fn test_remove_range_consolidate_1() {
        let mut s = NumericRangeSet::new();
        s.add(r(-6, -3));
        s.add(r(1, 10));
        s.add(r(15, 20));
        s.add(r(25, 30));
        s.remove(&r(0, 21));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(-6, -3), r(25, 30)]));
    }

    #[test]
    fn test_remove_range_consolidate_2() {
        let mut s = NumericRangeSet::new();
        s.add(r(-6, -3));
        s.add(r(1, 10));
        s.add(r(15, 20));
        s.add(r(25, 30));
        s.remove(&r(1, 20));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(-6, -3), r(25, 30)]));
    }

    #[test]
    fn test_remove_range_consolidate_3() {
        let mut s = NumericRangeSet::new();
        s.add(r(-6, -3));
        s.add(r(1, 10));
        s.add(r(15, 20));
        s.add(r(25, 30));
        s.remove(&r(5, 21));

        assert_invariants(&s);
        assert_eq!(as_vec(&s), kvs(&[r(-6, -3), r(1, 4), r(25, 30)]));
    }

    #[test]
    fn test_bounds() {
        let mut s = NumericRangeSet::new();
        s.add(r(0, 10));
        s.add(r(20, 30));

        assert_eq!(s.bounds(), r(0, ib(30)));
        assert_eq!(NumericRangeSet::new().bounds(), empty());
    }

    #[test]
    fn test_contains() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 3));
        s.add(r(5, 9));

        assert!(s.contains(1));
        assert!(s.contains(2));
        assert!(s.contains(3));
        assert!(s.contains(5));
        assert!(s.contains(7));
        assert!(s.contains(9));
        assert!(!s.contains(0));
        assert!(!s.contains(4));
        assert!(!s.contains(10));
    }

    #[test]
    fn test_contains_range() {
        let mut s = NumericRangeSet::new();
        s.add(r(0, 10));
        s.add(r(20, 30));

        assert!(s.contains_range(&r(1, 9)));
        assert!(s.contains_range(&r(1, 10)));
        assert!(s.contains_range(&r(0, 10)));
        assert!(s.contains_range(&r(0, 9)));

        assert!(s.contains_range(&r(21, 29)));
        assert!(s.contains_range(&r(20, 30)));
        assert!(s.contains_range(&r(21, 30)));
        assert!(s.contains_range(&r(20, 29)));

        assert!(!s.contains_range(&r(-5, -3)));
        assert!(!s.contains_range(&r(-5, 5)));
        assert!(!s.contains_range(&r(5, 11)));
        assert!(!s.contains_range(&r(13, 15)));
        assert!(!s.contains_range(&r(17, 22)));
        assert!(!s.contains_range(&r(17, 30)));

        assert!(!s.contains_range(&r(17, 31)));
        assert!(!s.contains_range(&r(-1, 11)));
        assert!(!s.contains_range(&r(-1, 31)));
    }

    #[test]
    fn test_contains_range_empty() {
        let mut s = NumericRangeSet::new();

        assert!(s.contains_range(&empty()));

        s.add(r(12, 69));

        assert!(s.contains_range(&empty()));
    }

    #[test]
    fn test_iter_range() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 3));
        s.add(r(11, 13));
        s.add(r(21, 23));

        assert_eq!(
            s.iter_range(&r(12, 22)).collect_vec(),
            vec!(r(11, 13), r(21, 23))
        );
        assert_eq!(
            s.iter_range(&r(11, 23)).collect_vec(),
            vec!(r(11, 13), r(21, 23))
        );
        assert_eq!(
            s.iter_range(&r(13, 21)).collect_vec(),
            vec!(r(11, 13), r(21, 23))
        );
        assert_eq!(s.iter_range(&r(23, 29)).collect_vec(), vec!(r(21, 23)));
        assert_eq!(s.iter_range(&r(25, 29)).collect_vec(), Vec::new());
    }

    #[test]
    fn test_intersects_range() {
        let mut s = NumericRangeSet::new();
        s.add(r(1, 3));
        s.add(r(11, 13));
        s.add(r(21, 23));

        assert!(s.intersects_range(&r(12, 15)));
        assert!(s.intersects_range(&r(13, 15)));
        assert!(s.intersects_range(&r(0, 30)));
        assert!(s.intersects_range(&empty()));
        assert!(!s.intersects_range(&r(15, 16)));
    }

    #[test]
    fn test_max() {
        let mut s = NumericRangeSet::new();
        s.add(r(0, 10));
        s.add(r(20, 30));

        assert_eq!(s.max(), Some(ib(30)));
    }

    #[test]
    fn test_min() {
        let mut s = NumericRangeSet::new();
        s.add(r(0, 10));
        s.add(r(20, 30));

        assert_eq!(s.min(), Some(ib(0)));
    }

    #[test]
    fn test_span() {
        let mut s = NumericRangeSet::new();
        s.add(r(5, 10));
        s.add(r(20, 30));

        assert_eq!(s.span(), Some(ub(25usize)));
    }

    proptest! {
        #[test]
        fn add_implies_contains(a in 1..1000, b in 1..1000) {
            prop_assume!(a <= b);

            let mut s = NumericRangeSet::new();

            s.add(r(a, b));
            prop_assert!(s.contains_range(&r(a, b)));
        }

        #[test]
        fn fuzz_add_many(seq in vec((1..1000, 1..1000), 1..100)) {
            let ranges = seq
                .into_iter()
                .map(|(a, b)| if a < b { r(a, b) } else { r(b, a) })
                .collect_vec();

            let mut s = NumericRangeSet::new();

            for element in ranges.clone() {
                s.add(element);
                assert_invariants(&s);
            }

            assert_invariants(&s);
            prop_assert_eq!(
                as_vec(&s),
                kvs(&consolidate_range_stream(ranges.into_iter()))
            );
        }

        #[test]
        fn fuzz_add_remove(seq in vec((1..1000, 1..1000, proptest::bool::ANY), 1..100)) {
            let mut s = NumericRangeSet::new();
            let mut hs = HashSet::new();

            for (a, b, remove) in seq {
                if remove {
                    s.remove(&r(a, b));
                    for num in r(a, b).iter() {
                        hs.remove(&num);
                    }
                } else {
                    s.add(r(a, b));
                    for num in r(a, b).iter() {
                        hs.insert(num);
                    }
                }
            }

            assert_invariants(&s);
            prop_assert_eq!(
                as_vec(&s).iter().flat_map(|x| x.1.iter()).collect_hashset(),
                hs
            );
        }

        #[test]
        fn fuzz_iter_range(seq in vec((1..1000, 1..1000), 1..100), lo in 1..1000, hi in 1..1000) {
            let s: NumericRangeSet = seq.iter().map(|t| r(t.0, t.1)).collect();

            assert_invariants(&s);

            let results = s.iter_range(&r(lo, hi)).collect_vec();

            let expected = consolidate_range_stream(seq.into_iter().map(|t| r(t.0, t.1)))
                .into_iter()
                .filter(|range| !range.disjoint_to(&r(lo, hi)))
                .collect_vec();

            prop_assert_eq!(results, expected);
        }
    }
}
