use crate::parasect::worker::WorkerMessage;
use crate::range::numeric_range::NumericRange;

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug)]
pub enum Event {
    WorkerMessageSent(WorkerMessage),
    ParasectCancelled(String),
    RangeInvalidated(NumericRange),
}
