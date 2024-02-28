use crate::collections::collect_collection::CollectVec;
use crate::messaging::listener::Listener;
use crate::parasect::event::Event;
use crate::parasect::event::Event::*;
use crate::parasect::types::ParasectPayloadAnswer::*;
use crate::parasect::types::ParasectPayloadResult::*;
use crate::parasect::types::{ParasectPayloadAnswer, ParasectPayloadResult};
use crate::parasect::worker::PointCompletionMessageType::*;
use crate::parasect::worker::WorkerMessage;
use crate::ui::line::mkline;
use crate::ui::line::Line;
use crate::ui::recent_log_display::LogType::*;
use crate::ui::segment::{Attributes, Color, Segment};
use crate::ui::ui_component::UiComponent;
use crossbeam_channel::Receiver;
use lru::LruCache;
use std::sync::{Arc, RwLock};

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug)]
enum LogType {
    Thread(usize),
    RangeInvalidation,
    Cancellation,
}

/// Displays the most recently updated logs from the given event stream.
///
/// Logs of the same type (logs for the same thread ID, range invalidations, cancellations) overwrite the previous entry of that type.
///
/// Produces `max(max_height, logs.len())` lines when rendered.
pub struct RecentLogDisplay {
    lru_logs: Arc<RwLock<LruCache<LogType, Event>>>,
    _event_listener: Listener<'static, Event>,
}

impl RecentLogDisplay {
    fn answer_segment(a: &ParasectPayloadAnswer) -> Segment {
        (
            a.to_string(),
            match a {
                Good => Color::Green,
                Bad => Color::Red,
            },
            Attributes::Bold,
        )
            .into()
    }

    fn result_segment(p: &ParasectPayloadResult) -> Segment {
        match p {
            Continue(a) => Self::answer_segment(&a),
            Stop(s) => (format!("Abort ({s})"), Color::Magenta, Attributes::Bold).into(),
        }
    }

    /// Makes a shorter log message if the terminal is too thin to render the long one.
    fn make_log_message_short(event: &Event) -> Line {
        match event {
            WorkerMessageSent(wm) => match &wm.msg_type {
                Started => mkline!(
                    wm.thread_id,
                    ": ",
                    (&wm.point, Color::Blue, Attributes::Bold)
                ),
                Completed(c) => mkline!(
                    wm.thread_id,
                    ": ",
                    (&wm.point, Color::Blue, Attributes::Bold),
                    " ",
                    Self::result_segment(&c)
                ),
                Cancelled => mkline!(wm.thread_id, ": ", ("cancelled", Color::Yellow)),
            },
            ParasectCancelled(reason) => mkline!(
                ("Aborting", Color::Magenta, Attributes::Bold),
                " ",
                (format!("({})", reason), Color::Magenta)
            ),
            RangeInvalidated(r, ans) => {
                mkline!(
                    "[",
                    (r.first().unwrap(), Color::Blue),
                    ", ",
                    (r.last().unwrap(), Color::Blue),
                    "]: known ",
                    Self::answer_segment(&ans)
                )
            }
        }
    }

    /// Makes the full log message for sufficiently wide terminals.
    fn make_log_message_long(event: &Event) -> Line {
        match event {
            WorkerMessageSent(wm) => match &wm.msg_type {
                Started => {
                    mkline!(
                        "Thread ",
                        wm.thread_id,
                        ": ",
                        ("working", Color::Yellow),
                        " x=",
                        (&wm.point, Color::Blue, Attributes::Bold),
                        " range=[",
                        (wm.left.first().unwrap_or(wm.point.clone()), Color::Blue),
                        ", ",
                        (wm.right.last().unwrap_or(wm.point.clone()), Color::Blue),
                        "]"
                    )
                }
                Completed(status) => {
                    mkline!(
                        "Thread ",
                        wm.thread_id,
                        ": ",
                        ("completed", Color::Green, Attributes::Bold),
                        " status=",
                        Self::result_segment(status),
                        " x=",
                        (&wm.point, Color::Blue, Attributes::Bold),
                        " range=[",
                        (wm.left.first().unwrap_or(wm.point.clone()), Color::Blue),
                        ", ",
                        (wm.right.last().unwrap_or(wm.point.clone()), Color::Blue),
                        "]"
                    )
                }
                Cancelled => {
                    mkline!(
                        "Thread ",
                        wm.thread_id,
                        ": ",
                        ("cancelled", Attributes::Bold),
                        " x=",
                        (&wm.point, Color::Blue, Attributes::Bold),
                        " range=[",
                        (wm.left.last().unwrap_or(wm.point.clone()), Color::Blue),
                        ", ",
                        (wm.right.last().unwrap_or(wm.point.clone()), Color::Blue),
                        "]"
                    )
                }
            },
            ParasectCancelled(reason) => {
                mkline!(
                    ("Parasect cancelled", Color::Magenta, Attributes::Bold),
                    ": ",
                    (reason, Color::Magenta)
                )
            }
            RangeInvalidated(r, ans) => {
                mkline!(
                    "[",
                    (r.first().unwrap(), Color::Blue),
                    ", ",
                    (r.last().unwrap(), Color::Blue),
                    "] known to be ",
                    Self::answer_segment(ans)
                )
            }
        }
    }

    /// Makes a log message properly sized to the given `width`.
    fn make_log_message(event: &Event, width: usize) -> Line {
        let long = Self::make_log_message_long(&event);
        if long.len() <= width {
            return long;
        }

        Self::make_log_message_short(&event).truncate(width)
    }

    fn event_log_type(event: &Event) -> LogType {
        match event {
            WorkerMessageSent(WorkerMessage { thread_id, .. }) => Thread(*thread_id),
            RangeInvalidated(_, _) => RangeInvalidation,
            ParasectCancelled(_) => Cancellation,
        }
    }

    pub fn new(event_receiver: Receiver<Event>) -> Self {
        let lru_logs = Arc::new(RwLock::new(LruCache::unbounded()));
        let lru_logs_clone = lru_logs.clone();

        Self {
            lru_logs,
            _event_listener: Listener::spawn(event_receiver, move |event| {
                let mut lru_write = lru_logs_clone.write().unwrap();

                let key = Self::event_log_type(&event);

                if lru_write.get(&key) != Some(&event) {
                    lru_write.put(Self::event_log_type(&event), event);
                }
            }),
        }
    }
}

impl UiComponent for RecentLogDisplay {
    fn render(&self, width: usize, max_height: usize) -> Vec<Line> {
        let lru_read = self.lru_logs.read().unwrap();

        lru_read
            .iter()
            .take(max_height)
            .map(|x| Self::make_log_message(x.1, width))
            .collect_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_util::test_util::{ib, r};
    use crossbeam_channel::unbounded;
    use std::thread;
    use std::time::Duration;

    pub fn test_wm() -> WorkerMessage {
        WorkerMessage {
            thread_id: 420,
            left: r(66, 68),
            point: ib(69),
            right: r(70, 72),
            msg_type: Started,
        }
    }

    pub fn log_display_from_events<I: IntoIterator<Item = Event>>(events: I) -> RecentLogDisplay {
        let (send, recv) = unbounded();

        for ev in events {
            send.send(ev).unwrap();
        }

        let log_display = RecentLogDisplay::new(recv.clone());

        while !recv.is_empty() {
            thread::sleep(Duration::from_millis(5));
        }

        log_display
    }

    #[test]
    pub fn test_make_log_message_short() {
        assert_eq!(
            RecentLogDisplay::make_log_message_short(&WorkerMessageSent(test_wm())),
            mkline!("420: ", ("69", Color::Blue, Attributes::Bold))
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_short(&WorkerMessageSent(WorkerMessage {
                msg_type: Completed(Continue(Good)),
                ..test_wm()
            })),
            mkline!(
                "420: ",
                ("69", Color::Blue, Attributes::Bold),
                " ",
                ("Good", Color::Green, Attributes::Bold)
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_short(&WorkerMessageSent(WorkerMessage {
                msg_type: Completed(Continue(Bad)),
                ..test_wm()
            })),
            mkline!(
                "420: ",
                ("69", Color::Blue, Attributes::Bold),
                " ",
                ("Bad", Color::Red, Attributes::Bold)
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_short(&WorkerMessageSent(WorkerMessage {
                msg_type: Completed(Stop("nope".into())),
                ..test_wm()
            })),
            mkline!(
                "420: ",
                ("69", Color::Blue, Attributes::Bold),
                " ",
                ("Abort (nope)", Color::Magenta, Attributes::Bold)
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_short(&RangeInvalidated(r(1, 5), Good)),
            mkline!(
                "[",
                ("1", Color::Blue),
                ", ",
                ("5", Color::Blue),
                "]: known ",
                ("Good", Color::Green, Attributes::Bold)
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_short(&RangeInvalidated(r(1, 5), Bad)),
            mkline!(
                "[",
                ("1", Color::Blue),
                ", ",
                ("5", Color::Blue),
                "]: known ",
                ("Bad", Color::Red, Attributes::Bold)
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_short(&ParasectCancelled("foobar".into())),
            mkline!(
                ("Aborting", Color::Magenta, Attributes::Bold),
                " ",
                ("(foobar)", Color::Magenta)
            )
        )
    }

    #[test]
    pub fn test_make_log_message_long() {
        assert_eq!(
            RecentLogDisplay::make_log_message_long(&WorkerMessageSent(test_wm())),
            mkline!(
                "Thread 420: ",
                ("working", Color::Yellow),
                " x=",
                ("69", Color::Blue, Attributes::Bold),
                " range=[",
                ("66", Color::Blue),
                ", ",
                ("72", Color::Blue),
                "]"
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_long(&WorkerMessageSent(WorkerMessage {
                msg_type: Completed(Continue(Good)),
                ..test_wm()
            })),
            mkline!(
                "Thread 420: ",
                ("completed", Color::Green, Attributes::Bold),
                " status=",
                ("Good", Color::Green, Attributes::Bold),
                " x=",
                ("69", Color::Blue, Attributes::Bold),
                " range=[",
                ("66", Color::Blue),
                ", ",
                ("72", Color::Blue),
                "]"
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_long(&WorkerMessageSent(WorkerMessage {
                msg_type: Completed(Continue(Bad)),
                ..test_wm()
            })),
            mkline!(
                "Thread 420: ",
                ("completed", Color::Green, Attributes::Bold),
                " status=",
                ("Bad", Color::Red, Attributes::Bold),
                " x=",
                ("69", Color::Blue, Attributes::Bold),
                " range=[",
                ("66", Color::Blue),
                ", ",
                ("72", Color::Blue),
                "]"
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_long(&WorkerMessageSent(WorkerMessage {
                msg_type: Completed(Stop("nope".into())),
                ..test_wm()
            })),
            mkline!(
                "Thread 420: ",
                ("completed", Color::Green, Attributes::Bold),
                " status=",
                ("Abort (nope)", Color::Magenta, Attributes::Bold),
                " x=",
                ("69", Color::Blue, Attributes::Bold),
                " range=[",
                ("66", Color::Blue),
                ", ",
                ("72", Color::Blue),
                "]"
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_long(&RangeInvalidated(r(1, 5), Good)),
            mkline!(
                "[",
                ("1", Color::Blue),
                ", ",
                ("5", Color::Blue),
                "] known to be ",
                ("Good", Color::Green, Attributes::Bold)
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_long(&RangeInvalidated(r(1, 5), Bad)),
            mkline!(
                "[",
                ("1", Color::Blue),
                ", ",
                ("5", Color::Blue),
                "] known to be ",
                ("Bad", Color::Red, Attributes::Bold)
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message_long(&ParasectCancelled("foobar".into())),
            mkline!(
                ("Parasect cancelled", Color::Magenta, Attributes::Bold),
                ": ",
                ("foobar", Color::Magenta)
            )
        )
    }

    #[test]
    pub fn test_make_log_message() {
        assert_eq!(
            RecentLogDisplay::make_log_message(&WorkerMessageSent(test_wm()), 80),
            mkline!(
                "Thread 420: ",
                ("working", Color::Yellow),
                " x=",
                ("69", Color::Blue, Attributes::Bold),
                " range=[",
                ("66", Color::Blue),
                ", ",
                ("72", Color::Blue),
                "]"
            )
        );

        assert_eq!(
            RecentLogDisplay::make_log_message(&WorkerMessageSent(test_wm()), 10),
            mkline!("420: ", ("69", Color::Blue, Attributes::Bold))
        );

        assert_eq!(
            RecentLogDisplay::make_log_message(&WorkerMessageSent(test_wm()), 5),
            mkline!("420:…")
        );
    }

    #[test]
    pub fn test_event_log_type() {
        let t1 = RecentLogDisplay::event_log_type(&WorkerMessageSent(test_wm()));
        let t2 = RecentLogDisplay::event_log_type(&WorkerMessageSent(WorkerMessage {
            thread_id: 69,
            ..test_wm()
        }));

        let r1 = RecentLogDisplay::event_log_type(&RangeInvalidated(r(1, 2), Good));
        let r2 = RecentLogDisplay::event_log_type(&RangeInvalidated(r(3, 4), Bad));

        let c1 = RecentLogDisplay::event_log_type(&ParasectCancelled("foo".into()));
        let c2 = RecentLogDisplay::event_log_type(&ParasectCancelled("bar".into()));

        assert_ne!(t1, t2);
        assert_eq!(r1, r2);
        assert_eq!(c1, c2);
        assert_ne!(t1, r1);
        assert_ne!(t1, c1);
        assert_ne!(r1, c1);
    }

    #[test]
    pub fn test_new() {
        let evs = [
            WorkerMessageSent(WorkerMessage {
                thread_id: 0,
                left: r(0, 19),
                point: ib(20),
                right: r(21, 40),
                msg_type: Started,
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 1,
                left: r(0, 9),
                point: ib(10),
                right: r(11, 19),
                msg_type: Started,
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 2,
                left: r(20, 29),
                point: ib(30),
                right: r(31, 40),
                msg_type: Started,
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 0,
                left: r(0, 19),
                point: ib(20),
                right: r(21, 40),
                msg_type: Completed(Continue(Bad)),
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 1,
                left: r(0, 9),
                point: ib(10),
                right: r(11, 19),
                msg_type: Completed(Continue(Good)),
            }),
            RangeInvalidated(r(20, 40), Bad),
            RangeInvalidated(r(0, 10), Good),
            WorkerMessageSent(WorkerMessage {
                thread_id: 2,
                left: r(20, 29),
                point: ib(30),
                right: r(31, 40),
                msg_type: Cancelled,
            }),
        ];

        let log_display = log_display_from_events(evs.clone());
        thread::sleep(Duration::from_millis(5));

        let logs = log_display
            .lru_logs
            .read()
            .unwrap()
            .iter()
            .map(|x| (x.0.clone(), x.1.clone()))
            .collect_vec();

        let expected = [
            (Thread(2), evs[7].clone()),
            (RangeInvalidation, evs[6].clone()),
            (Thread(1), evs[4].clone()),
            (Thread(0), evs[3].clone()),
        ];

        assert_eq!(logs, expected.into_iter().collect_vec());
    }

    #[test]
    pub fn test_render() {
        let evs = [
            WorkerMessageSent(WorkerMessage {
                thread_id: 0,
                left: r(0, 19),
                point: ib(20),
                right: r(21, 40),
                msg_type: Started,
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 1,
                left: r(0, 9),
                point: ib(10),
                right: r(11, 19),
                msg_type: Started,
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 2,
                left: r(20, 29),
                point: ib(30),
                right: r(31, 40),
                msg_type: Started,
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 0,
                left: r(0, 19),
                point: ib(20),
                right: r(21, 40),
                msg_type: Completed(Continue(Bad)),
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 1,
                left: r(0, 9),
                point: ib(10),
                right: r(11, 19),
                msg_type: Completed(Continue(Good)),
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 2,
                left: r(20, 29),
                point: ib(30),
                right: r(31, 40),
                msg_type: Cancelled,
            }),
            RangeInvalidated(r(20, 40), Bad),
            RangeInvalidated(r(0, 10), Good),
            WorkerMessageSent(WorkerMessage {
                thread_id: 0,
                left: r(11, 14),
                point: ib(15),
                right: r(16, 19),
                msg_type: Started,
            }),
            WorkerMessageSent(WorkerMessage {
                thread_id: 0,
                left: r(11, 14),
                point: ib(15),
                right: r(16, 19),
                msg_type: Completed(Stop("nope".into())),
            }),
            ParasectCancelled("nope".into()),
        ];

        let log_display = log_display_from_events(evs.clone());
        thread::sleep(Duration::from_millis(5));

        let rows = log_display.render(30, 3);
        let expected = [
            mkline!(
                ("Parasect cancelled", Color::Magenta, Attributes::Bold),
                ": ",
                ("nope", Color::Magenta)
            ),
            mkline!(
                "0: ",
                ("15", Color::Blue, Attributes::Bold),
                " ",
                ("Abort (nope)", Color::Magenta, Attributes::Bold)
            ),
            mkline!(
                "[",
                ("0", Color::Blue),
                ", ",
                ("10", Color::Blue),
                "] known to be ",
                ("Good", Color::Green, Attributes::Bold)
            ),
        ];

        assert_eq!(expected.into_iter().collect_vec(), rows);
    }
}
