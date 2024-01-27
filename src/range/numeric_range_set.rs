use std::collections::BTreeMap;
use std::ops::Bound::Included;
use ibig::IBig;
use crate::range::numeric_range::NumericRange;

/// A set of continuous ranges of integers.
/// This can represent any subset of [-inf, inf].
pub struct NumericRangeSet {
    range_starts: BTreeMap<IBig, NumericRange>
}

impl NumericRangeSet {
    pub fn new() -> Self {
        Self { range_starts: BTreeMap::new() }
    }

    pub fn add(&mut self, range: NumericRange) {
        if range.is_empty() {
            return;
        }

        let key = range.first().unwrap();

        let mut cursor = self.range_starts.upper_bound_mut(
            Included(&key)
        );

        loop {
            match cursor.value_mut() {
                None => {
                    cursor.insert_after(key, range);
                    return;
                },
                Some(cursor_range) => {
                    match &range | &*cursor_range {
                        x => todo!()
                    }
                }
            }
        }
    }
}