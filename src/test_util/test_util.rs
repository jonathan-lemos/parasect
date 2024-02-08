use crate::range::numeric_range::NumericRange;

#[cfg(test)]
pub mod test_util {
    use crate::range::numeric_range::NumericRange;
    use ibig::{IBig, UBig};

    pub fn detect_flake<F: FnMut() -> ()>(mut f: F) {
        for _ in 0..5000 {
            f();
        }
    }

    pub fn ib<A: Into<IBig>>(n: A) -> IBig {
        n.into()
    }

    pub fn ub<A: Into<UBig>>(n: A) -> UBig {
        n.into()
    }

    pub fn empty() -> NumericRange {
        NumericRange::empty()
    }

    pub fn r<A: Into<IBig>, B: Into<IBig>>(low: A, high: B) -> NumericRange {
        NumericRange::from_endpoints_inclusive(low, high)
    }
}
