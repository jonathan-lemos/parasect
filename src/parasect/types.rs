use crate::parasect::types::ParasectPayloadAnswer::*;
use crate::parasect::types::ParasectPayloadResult::*;
use std::fmt::{Debug, Display, Formatter};

#[derive(PartialEq, Eq, Ord, PartialOrd, Hash, Copy, Clone, Debug)]
pub enum ParasectPayloadAnswer {
    Good,
    Bad,
}

impl Display for ParasectPayloadAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Good => f.write_str("Good"),
            Bad => f.write_str("Bad"),
        }?;
        Ok(())
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum ParasectPayloadResult {
    Continue(ParasectPayloadAnswer),
    Stop(String),
}

impl Display for ParasectPayloadResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Continue(ans) => f.write_str(&format!("{}", ans)),
            Stop(s) => f.write_str(&format!("Aborting ({})", s)),
        }?;
        Ok(())
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum ParasectError {
    PayloadError(String),
    InconsistencyError(String),
}
