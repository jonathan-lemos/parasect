#[derive(PartialEq, Eq, Ord, PartialOrd, Hash, Copy, Clone, Debug)]
pub enum ParasectPayloadAnswer {
    Good,
    Bad,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum ParasectPayloadResult {
    Continue(ParasectPayloadAnswer),
    Stop(String),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum ParasectError {
    PayloadError(String),
    InconsistencyError(String),
}
