use crate::parasect::event::Event;
use crate::parasect::event::Event::*;
use crate::parasect::types::ParasectPayloadAnswer::*;
use crate::parasect::types::ParasectPayloadResult::*;
use crate::parasect::worker::PointCompletionMessageType::*;
use crate::range::numeric_range::NumericRange;
use crate::threading::background_loop::BackgroundLoop;
use crate::threading::background_loop::BackgroundLoopBehavior::DontCancel;
use crossbeam_channel::Receiver;

/// Instantiation of this struct outputs a stream of logs to stdout.
///
/// There should be no more than one instance of either `NoTtyUi` or `TtyUi` at any time.
///
/// The logs stops outputting when this struct is dropped.
pub struct NoTtyUi {
    _receiver_loop: BackgroundLoop,
}

impl NoTtyUi {
    fn make_log_message(e: &Event) -> String {
        match e {
            WorkerMessageSent(wm) => match &wm.msg_type {
                Started => format!(
                    "[Thread {}] Started processing point {}, between left half {} and right half {}.",
                    wm.thread_id, wm.point, wm.left, wm.right
                ),
                Completed(Continue(v)) => format!(
                    "[Thread {}] Finished processing point {}, result was {}.",
                    wm.thread_id, wm.point, v
                ),
                Completed(Stop(msg)) => format!(
                    "[Thread {}] Finished processing point {}, aborting parasect execution for reason: {}.",
                    wm.thread_id, wm.point, msg
                ),
                Cancelled => format!(
                    "[Thread {}] Cancelled processing point {} because it is in a range that has been eliminated.",
                    wm.thread_id, wm.point
                ),
            }
            RangeInvalidated(range, answer) => format!(
                "Eliminating range {}. It is known {}.",
                range, answer
            ),
            ParasectCancelled(msg) => format!(
                "[FATAL] Aborting parasect execution for reason: {}",
                msg
            )
        }
    }

    pub fn start(
        initial_range: NumericRange,
        command_string: String,
        event_receiver: Receiver<Event>,
    ) -> Self {
        println!("Parasecting over range {}", initial_range);
        println!("Command: {}", command_string);
        Self {
            _receiver_loop: BackgroundLoop::spawn(event_receiver, |event| {
                println!("{}", Self::make_log_message(&event));
                DontCancel
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parasect::worker::WorkerMessage;
    use crate::test_util::test_util::test_util::{ib, r};

    #[test]
    fn test_make_log_message() {
        assert_eq!(
            NoTtyUi::make_log_message(&WorkerMessageSent(WorkerMessage {
                thread_id: 5,
                left: r(1, 3),
                point: ib(4),
                right: r(5, 7),
                msg_type: Started,
            })),
            "[Thread 5] Started processing point 4, between left half [1, 3] and right half [5, 7]."
        );

        assert_eq!(
            NoTtyUi::make_log_message(&WorkerMessageSent(WorkerMessage {
                thread_id: 5,
                left: r(1, 3),
                point: ib(4),
                right: r(5, 7),
                msg_type: Completed(Continue(Good)),
            })),
            "[Thread 5] Finished processing point 4, result was Good."
        );

        assert_eq!(
            NoTtyUi::make_log_message(&WorkerMessageSent(WorkerMessage {
                thread_id: 5,
                left: r(1, 3),
                point: ib(4),
                right: r(5, 7),
                msg_type: Completed(Continue(Bad)),
            })),
            "[Thread 5] Finished processing point 4, result was Bad."
        );

        assert_eq!(
            NoTtyUi::make_log_message(&WorkerMessageSent(WorkerMessage {
                thread_id: 5,
                left: r(1, 3),
                point: ib(4),
                right: r(5, 7),
                msg_type: Completed(Stop("nope".into())),
            })),
            "[Thread 5] Finished processing point 4, aborting parasect execution for reason: nope."
        );

        assert_eq!(
            NoTtyUi::make_log_message(&WorkerMessageSent(WorkerMessage {
                thread_id: 5,
                left: r(1, 3),
                point: ib(4),
                right: r(5, 7),
                msg_type: Cancelled,
            })),
            "[Thread 5] Cancelled processing point 4 because it is in a range that has been eliminated."
        );

        assert_eq!(
            NoTtyUi::make_log_message(&RangeInvalidated(r(1, 3), Good)),
            "Eliminating range [1, 3]. It is known Good."
        );

        assert_eq!(
            NoTtyUi::make_log_message(&ParasectCancelled("nope".into())),
            "[FATAL] Aborting parasect execution for reason: nope"
        );
    }
}
