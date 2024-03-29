#[cfg(test)]
pub mod test_util {
    use crate::range::numeric_range::NumericRange;
    use ibig::{IBig, UBig};
    use std::thread;
    use std::time::{Duration, Instant};

    pub fn ib(n: impl Into<IBig>) -> IBig {
        n.into()
    }

    pub fn ub(n: impl Into<UBig>) -> UBig {
        n.into()
    }

    pub fn empty() -> NumericRange {
        NumericRange::empty()
    }

    pub fn r(low: impl Into<IBig>, high: impl Into<IBig>) -> NumericRange {
        NumericRange::from_endpoints_inclusive(low, high)
    }

    pub fn wait_for_condition(
        mut condition: impl FnMut() -> bool,
        timeout: Duration,
        timeout_msg: impl ToString,
    ) {
        let start = Instant::now();

        while !condition() {
            if Instant::now() - start > timeout {
                panic!("{}", timeout_msg.to_string());
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}
