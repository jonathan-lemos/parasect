use std::cmp::{max, min};
use std::fmt::{Debug, Display, Formatter, Write};
use std::ops::{BitAnd, BitOr, Sub};
use ibig::{IBig, UBig};
use ibig::ops::Abs;
use crate::range::numeric_range::MaybeSplitNumericRange::*;

/// A continuous range of integers.
// low and high are inclusive. If low > high, then (low, high) must equal (0, -1)
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Hash)]
pub struct NumericRange {
    low: IBig,
    high: IBig,
}

impl Debug for NumericRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            f.write_str("NumericRange::empty()")
        } else {
            f.write_str(&format!("NumericRange::from_endpoints_inclusive({:?}, {:?})", &self.low, &self.high))
        }
    }
}

impl Display for NumericRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            f.write_str("∅")
        } else {
            f.write_str(&format!("[{}, {}]", &self.low, &self.high))
        }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Debug)]
pub enum MaybeSplitNumericRange {
    NotSplit(NumericRange),
    Split(NumericRange, NumericRange),
}

impl MaybeSplitNumericRange {
    /// Returns a vector of the non-empty ranges in `self`.
    ///
    /// The returned vector may have 0, 1, or 2 elements.
    pub fn as_vec(&self) -> Vec<NumericRange> {
        return match self {
            Split(r1, r2) =>
                [r1, r2].into_iter()
                    .filter(|x| !x.is_empty())
                    .map(|x| x.clone())
                    .collect(),
            NotSplit(r) =>
                if r.is_empty() {
                    vec!()
                } else {
                    vec!(r.clone())
                }
        };
    }
}

fn split_not_empty(a: NumericRange, b: NumericRange) -> MaybeSplitNumericRange {
    if b.is_empty() {
        NotSplit(a)
    } else if a.is_empty() {
        NotSplit(b)
    } else if a.low < b.low {
        Split(a, b)
    } else {
        Split(b, a)
    }
}

impl NumericRange {
    /// A tuple of the *inclusive* endpoints of this range.
    pub fn as_tuple(&self) -> Option<(IBig, IBig)> {
        self.first().and_then(|x| self.last().map(|y| (x, y)))
    }

    /// `true` if the given number is in the range.
    pub fn contains<A: Into<IBig>>(&self, num: A) -> bool {
        if self.is_empty() {
            return false;
        }

        let num = num.into();
        return &self.low <= &num && &num <= &self.high;
    }

    /// `true` if *all* of the numbers in the given range are in `self`.
    ///
    /// Every range contains the empty range. An empty range only contains itself.
    pub fn contains_range(&self, range: &Self) -> bool {
        if range.is_empty() {
            return true;
        }

        if self.is_empty() {
            return false;
        }

        return self.low <= range.low && range.high <= self.high;
    }

    pub fn disjoint_to(&self, other: &NumericRange) -> bool {
        (self & other).is_empty()
    }

    /// A range with no numbers in it.
    pub fn empty() -> Self {
        Self {
            low: IBig::from(0),
            high: IBig::from(-1),
        }
    }

    /// The first number in this range, if it's not empty.
    pub fn first(&self) -> Option<IBig> {
        if self.is_empty() {
            None
        } else {
            Some(self.low.clone())
        }
    }

    /// Makes a range of numbers from [low, high).
    pub fn from_endpoints_excluding_end<A: Into<IBig>, B: Into<IBig>>(low: A, high: B) -> Self {
        let low = low.into();
        let high = high.into();

        if low >= high {
            Self::empty()
        } else {
            Self {
                low,
                high: high - 1,
            }
        }
    }

    /// Makes a range of numbers from [low, high].
    pub fn from_endpoints_inclusive<A: Into<IBig>, B: Into<IBig>>(low: A, high: B) -> Self {
        let low = low.into();
        let high = high.into();

        if low > high {
            Self::empty()
        } else {
            Self {
                low: low.into(),
                high: high.into(),
            }
        }
    }

    /// Makes a range of a single number.
    pub fn from_point<A: Into<IBig>>(num: A) -> Self {
        let low = num.into();
        let high = low.clone();

        Self {
            low,
            high,
        }
    }

    /// Makes a range of [low, low + length)
    pub fn from_point_and_length<A: Into<IBig>, B: Into<IBig>>(low: IBig, length: UBig) -> Self {
        let low: IBig = low.into();
        let length: UBig = length.into();

        if length == UBig::from(0usize) {
            return Self::empty();
        }

        let high: IBig = &low + IBig::from(length);

        Self {
            low,
            high,
        }
    }

    /// `true` if the range has no numbers in it.
    pub fn is_empty(&self) -> bool {
        self.len() == IBig::from(0)
    }

    /// The last number in the range, if it's not empty.
    pub fn last(&self) -> Option<IBig> {
        if self.is_empty() {
            None
        } else {
            Some(self.high.clone())
        }
    }

    /// How many numbers there are in the range.
    pub fn len(&self) -> IBig {
        max(IBig::from(0), &self.high - &self.low + 1)
    }
}

impl<'a, 'b> Sub<&'b NumericRange> for &'a NumericRange {
    type Output = MaybeSplitNumericRange;

    /// Returns this range with the given range removed.
    ///
    /// This may return a single range, or two ranges if the removed part is in the middle.
    fn sub(self, rhs: &'b NumericRange) -> Self::Output {
        if self.is_empty() || rhs.is_empty() || (self & rhs).is_empty() {
            return NotSplit(self.clone());
        }

        if self.low <= rhs.low && rhs.high <= self.high {
            return split_not_empty(
                NumericRange::from_endpoints_inclusive(self.low.clone(), &rhs.low - 1),
                NumericRange::from_endpoints_inclusive(&rhs.high + 1, self.high.clone()),
            );
        }

        if rhs.low <= self.low && self.high <= rhs.high {
            return NotSplit(NumericRange::empty());
        }

        if self.low <= rhs.low && self.high <= rhs.high {
            return NotSplit(NumericRange::from_endpoints_inclusive(self.low.clone(), &rhs.low - 1));
        }

        if rhs.low <= self.low && rhs.high <= self.high {
            return NotSplit(NumericRange::from_endpoints_inclusive(&rhs.high + 1, self.high.clone()));
        }

        panic!("should never happen")
    }
}

impl<'a> Sub<NumericRange> for &'a NumericRange {
    type Output = MaybeSplitNumericRange;

    /// Returns this range with the given range removed.
    ///
    /// This may return a single range, or two ranges if the removed part is in the middle.
    fn sub(self, rhs: NumericRange) -> Self::Output {
        self - &rhs
    }
}

impl<'a> Sub<&'a NumericRange> for NumericRange {
    type Output = MaybeSplitNumericRange;

    /// Returns this range with the given range removed.
    ///
    /// This may return a single range, or two ranges if the removed part is in the middle.
    fn sub(self, rhs: &'a NumericRange) -> Self::Output {
        &self - rhs
    }
}

impl Sub<NumericRange> for NumericRange {
    type Output = MaybeSplitNumericRange;

    /// Returns this range with the given range removed.
    ///
    /// This may return a single range, or two ranges if the removed part is in the middle.
    fn sub(self, rhs: NumericRange) -> Self::Output {
        &self - &rhs
    }
}

impl<'a, I: Into<IBig>> Sub<I> for &'a NumericRange {
    type Output = MaybeSplitNumericRange;

    /// Returns this range with the given integer removed.
    ///
    /// This may return a single range, or two ranges if the number is removed from the middle.
    fn sub(self, rhs: I) -> Self::Output {
        self - NumericRange::from_point(rhs)
    }
}

impl<I: Into<IBig>> Sub<I> for NumericRange {
    type Output = MaybeSplitNumericRange;

    /// Returns this range with the given integer removed.
    ///
    /// This may return a single range, or two ranges if the number is removed from the middle.
    fn sub(self, rhs: I) -> Self::Output {
        &self - NumericRange::from_point(rhs)
    }
}

impl<'a, 'b> BitAnd<&'b NumericRange> for &'a NumericRange {
    type Output = NumericRange;

    /// Returns the intersection, meaning the range of numbers in common, between this range and the other.
    fn bitand(self, rhs: &'b NumericRange) -> Self::Output {
        if self.is_empty() || rhs.is_empty() || self.high < rhs.low || self.low > rhs.high {
            return NumericRange::empty();
        }

        let mut points = vec!(&self.low, &self.high, &rhs.low, &rhs.high);
        points.sort();

        NumericRange::from_endpoints_inclusive(
            (*points.get(1).unwrap()).clone(),
            (*points.get(2).unwrap()).clone(),
        )
    }
}

impl<'a> BitAnd<NumericRange> for &'a NumericRange {
    type Output = NumericRange;

    /// Returns the intersection, meaning the range of numbers in common, between this range and the other.
    fn bitand(self, rhs: NumericRange) -> Self::Output {
        self & &rhs
    }
}

impl<'a> BitAnd<&'a NumericRange> for NumericRange {
    type Output = NumericRange;

    /// Returns the intersection, meaning the range of numbers in common, between this range and the other.
    fn bitand(self, rhs: &'a NumericRange) -> Self::Output {
        &self & rhs
    }
}

impl BitAnd<NumericRange> for NumericRange {
    type Output = NumericRange;

    /// Returns the intersection of the inputs, meaning the range of numbers in common.
    fn bitand(self, rhs: NumericRange) -> Self::Output {
        &self & &rhs
    }
}

impl<'a, 'b> BitOr<&'b NumericRange> for &'a NumericRange {
    type Output = MaybeSplitNumericRange;

    /// Returns the union of the inputs, meaning the range/ranges of numbers in either input.
    fn bitor(self, rhs: &'b NumericRange) -> Self::Output {
        if self.disjoint_to(rhs) && &self.high + 1 != rhs.low && &rhs.high + 1 != self.low {
            split_not_empty(self.clone(), rhs.clone())
        } else {
            NotSplit(
                NumericRange::from_endpoints_inclusive(
                    min(&self.low, &rhs.low).clone(),
                    max(&self.high, &rhs.high).clone(),
                )
            )
        }
    }
}

impl<'a> BitOr<NumericRange> for &'a NumericRange {
    type Output = MaybeSplitNumericRange;

    fn bitor(self, rhs: NumericRange) -> Self::Output {
        self | &rhs
    }
}

impl<'a> BitOr<&'a NumericRange> for NumericRange {
    type Output = MaybeSplitNumericRange;

    fn bitor(self, rhs: &'a NumericRange) -> Self::Output {
        &self | rhs
    }
}

impl BitOr<NumericRange> for NumericRange {
    type Output = MaybeSplitNumericRange;

    fn bitor(self, rhs: NumericRange) -> Self::Output {
        &self | &rhs
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::test_util::test_util::ib;
    use super::*;

    fn empty() -> NumericRange {
        NumericRange::empty()
    }

    fn r<A: Into<IBig>, B: Into<IBig>>(low: A, high: B) -> NumericRange {
        NumericRange::from_endpoints_inclusive(low, high)
    }

    #[test]
    fn test_as_tuple() {
        assert_eq!(None, empty().as_tuple());
        assert_eq!(Some((ib(1), ib(69))), r(1, 69).as_tuple())
    }

    #[test]
    fn test_contains() {
        assert!(r(69, 79).contains(69));
        assert!(r(69, 79).contains(71));
        assert!(r(69, 79).contains(79));
    }

    #[quickcheck]
    fn test_contains_always_has_endpoints(a: i32, b: i32) {
        let (a, b) = if (a < b) { (a, b) } else { (b, a) };
        let range = r(a, b);

        assert!(range.contains(a));
        assert!(range.contains(b));
    }

    #[quickcheck]
    fn test_contains_empty_always_false(n: i32) {
        assert!(!empty().contains(n));
    }

    #[test]
    fn test_contains_range() {
        assert!(r(1, 20).contains_range(&r(5, 15)));
        assert!(!r(1, 20).contains_range(&r(0, 1)));
        assert!(!r(1, 20).contains_range(&r(-5, -2)));
        assert!(!r(1, 20).contains_range(&r(23, 25)));
    }

    #[quickcheck]
    fn test_contains_range_contains_self(a: i32, b: i32) {
        assert!(r(a, b).contains_range(&r(a, b)));
    }

    #[quickcheck]
    fn test_contains_range_contains_empty(a: i32, b: i32) {
        assert!(r(a, b).contains_range(&empty()));
    }

    #[quickcheck]
    fn test_contains_range_contains_endpoints(a: i32, b: i32) {
        let (a, b) = if (a < b) { (a, b) } else { (b, a) };

        assert!(r(a, b).contains_range(&r(a, a)));
        assert!(r(a, b).contains_range(&r(b, b)));
    }

    #[test]
    fn test_empty() {
        assert!(empty().is_empty());
        assert_eq!(empty().len(), ib(0));
    }

    #[test]
    fn test_first() {
        assert_eq!(None, empty().first());
        assert_eq!(Some(ib(69)), r(69, 99).first());
    }

    #[test]
    fn test_from_endpoints_excluding_end() {
        assert_eq!(empty(), NumericRange::from_endpoints_excluding_end(69, 69));
        assert_eq!(empty(), NumericRange::from_endpoints_excluding_end(69, 68));
        assert_eq!(r(69, 69), NumericRange::from_endpoints_excluding_end(69, 70));
        assert_eq!(NumericRange::from_endpoints_excluding_end(69, 70).first(), Some(ib(69)));
        assert_eq!(NumericRange::from_endpoints_excluding_end(69, 70).last(), Some(ib(69)));
        assert_eq!(NumericRange::from_endpoints_excluding_end(69, 99).first(), Some(ib(69)));
        assert_eq!(NumericRange::from_endpoints_excluding_end(69, 99).last(), Some(ib(98)));
    }

    #[test]
    fn test_from_endpoints_inclusive() {
        assert_eq!(empty(), NumericRange::from_endpoints_inclusive(69, 68));
        assert_eq!(NumericRange::from_endpoints_inclusive(69, 69).first(), Some(ib(69)));
        assert_eq!(NumericRange::from_endpoints_inclusive(69, 69).last(), Some(ib(69)));
        assert_eq!(NumericRange::from_endpoints_inclusive(69, 100).first(), Some(ib(69)));
        assert_eq!(NumericRange::from_endpoints_inclusive(69, 100).last(), Some(ib(100)));
    }

    #[test]
    fn test_from_point() {
        assert_eq!(r(69, 69), NumericRange::from_point(69));
        assert_eq!(NumericRange::from_point(69).len(), ib(1));
    }

    #[test]
    fn test_is_empty() {
        assert!(empty().is_empty());
        assert!(r(69, 68).is_empty());
        assert!(!r(69, 69).is_empty());
    }

    #[test]
    fn test_last() {
        assert_eq!(None, empty().last());
        assert_eq!(Some(ib(99)), r(69, 99).last());
    }

    #[test]
    fn test_len() {
        assert_eq!(empty().len(), ib(0));
        assert_eq!(r(1, 10).len(), ib(10));
    }

    #[test]
    fn test_sub() {
        assert_eq!(
            r(1, 10) - 5,
            Split(
                r(1, 4),
                r(6, 10),
            )
        );

        assert_eq!(
            r(1, 10) - 0,
            NotSplit(
                r(1, 10),
            )
        );

        assert_eq!(
            r(1, 10) - 11,
            NotSplit(
                r(1, 10),
            )
        );

        assert_eq!(
            r(1, 10) - 1,
            NotSplit(
                r(2, 10),
            )
        );

        assert_eq!(
            r(1, 10) - 10,
            NotSplit(
                r(1, 9),
            )
        );

        assert_eq!(
            r(1, 10) - r(4, 6),
            Split(
                r(1, 3),
                r(7, 10),
            )
        );

        assert_eq!(
            r(1, 10) - r(0, 5),
            NotSplit(
                r(6, 10),
            )
        );

        assert_eq!(
            r(1, 10) - r(7, 15),
            NotSplit(
                r(1, 6),
            )
        );

        assert_eq!(
            r(1, 10) - r(-1, 15),
            NotSplit(
                empty(),
            )
        );

        assert_eq!(
            r(1, 10) - r(1, 5),
            NotSplit(
                r(6, 10),
            )
        );

        assert_eq!(
            r(1, 10) - r(5, 10),
            NotSplit(
                r(1, 4),
            )
        );

        assert_eq!(
            r(1, 10) - r(1, 10),
            NotSplit(
                empty(),
            )
        );

        assert_eq!(
            r(1, 10) - r(-20, -1),
            NotSplit(
                r(1, 10),
            )
        );

        assert_eq!(
            r(1, 10) - r(11, 15),
            NotSplit(
                r(1, 10),
            )
        );

        assert_eq!(
            r(1, 10) - r(-10, 1),
            NotSplit(
                r(2, 10),
            )
        );

        assert_eq!(
            r(1, 10) - r(10, 15),
            NotSplit(
                r(1, 9),
            )
        );

        assert_eq!(
            empty() - empty(),
            NotSplit(
                empty(),
            )
        );

        assert_eq!(
            empty() - r(69, 69),
            NotSplit(
                empty(),
            )
        );
    }

    #[quickcheck]
    fn test_sub_self_is_empty(a: i32, b: i32) {
        let range = r(a, b);
        assert_eq!(&range - &range, NotSplit(empty()));
    }

    #[quickcheck]
    fn test_sub_does_not_panic(a: i32, b: i32, c: i32, d: i32) {
        let r1 = r(a, b);
        let r2 = r(c, d);

        let _ = r1 - r2;
    }

    #[test]
    fn test_bitand() {
        assert_eq!(
            r(1, 10) & r(2, 8),
            r(2, 8)
        );

        assert_eq!(
            r(1, 10) & r(3, 12),
            r(3, 10)
        );

        assert_eq!(
            r(1, 10) & r(-5, 4),
            r(1, 4)
        );

        assert_eq!(
            r(1, 10) & r(-5, -1),
            empty()
        );

        assert_eq!(
            r(1, 10) & r(11, 15),
            empty()
        );

        assert_eq!(
            r(1, 10) & r(0, 12),
            r(1, 10)
        );

        assert_eq!(
            r(1, 10) & r(1, 10),
            r(1, 10)
        );

        assert_eq!(
            r(1, 10) & r(1, 5),
            r(1, 5)
        );

        assert_eq!(
            r(1, 10) & r(5, 10),
            r(5, 10)
        );

        assert_eq!(
            r(1, 10) & r(-1, 1),
            r(1, 1)
        );

        assert_eq!(
            r(1, 10) & r(10, 10),
            r(10, 10)
        );
    }

    #[quickcheck]
    fn test_bitand_empty_is_empty(a: i32, b: i32) {
        assert_eq!(r(a, b) & empty(), empty());
    }

    #[quickcheck]
    fn test_bitand_self_is_self(a: i32, b: i32) {
        let range = r(a, b);
        assert_eq!(&range & &range, range);
    }

    #[quickcheck]
    fn test_bitand_commutative(a: i32, b: i32, c: i32, d: i32) {
        let r1 = r(a, b);
        let r2 = r(c, d);
        assert_eq!(&r1 & &r2, &r2 & &r1);
    }

    #[test]
    fn test_bitor() {
        assert_eq!(
            r(1, 10) | r(2, 8),
            NotSplit(r(1, 10))
        );

        assert_eq!(
            r(1, 10) | r(3, 12),
            NotSplit(r(1, 12))
        );

        assert_eq!(
            r(1, 10) | r(-5, 4),
            NotSplit(r(-5, 10))
        );

        assert_eq!(
            r(1, 10) | r(-5, -1),
            Split(
                r(-5, -1), r(1, 10)
            )
        );

        assert_eq!(
            r(1, 10) | r(11, 15),
            NotSplit(r(1, 15))
        );

        assert_eq!(
            r(1, 10) | r(-10, 0),
            NotSplit(r(-10, 10))
        );

        assert_eq!(
            r(1, 10) | r(0, 12),
            NotSplit(r(0, 12))
        );

        assert_eq!(
            r(1, 10) | r(1, 10),
            NotSplit(r(1, 10))
        );

        assert_eq!(
            r(1, 10) | r(1, 5),
            NotSplit(r(1, 10))
        );

        assert_eq!(
            r(1, 10) | r(5, 10),
            NotSplit(r(1, 10))
        );

        assert_eq!(
            r(1, 10) | r(-1, 1),
            NotSplit(r(-1, 10))
        );

        assert_eq!(
            r(1, 10) | r(10, 10),
            NotSplit(r(1, 10))
        );
    }

    #[quickcheck]
    fn test_bitor_empty_is_self(a: i32, b: i32) {
        assert_eq!(r(a, b) | empty(), NotSplit(r(a, b)));
    }

    #[quickcheck]
    fn test_bitor_self_is_self(a: i32, b: i32) {
        let range = r(a, b);
        assert_eq!(&range | &range, NotSplit(range));
    }

    #[quickcheck]
    fn test_bitor_commutative(a: i32, b: i32, c: i32, d: i32) {
        let r1 = r(a, b);
        let r2 = r(c, d);
        assert_eq!(&r1 | &r2, &r2 | &r1);
    }
}