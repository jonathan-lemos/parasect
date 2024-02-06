#[cfg(test)]
pub mod test_util {
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
}