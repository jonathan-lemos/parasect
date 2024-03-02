use crate::collections::collect_collection::CollectVec;
use crate::messaging::fan::Fan;
use crate::messaging::listener::Listener;
use crate::messaging::periodic_notifier::PeriodicNotifier;
use crate::parasect::event::Event;
use crate::range::numeric_range::NumericRange;
use crate::ui::line::Line;
use crate::ui::progress_bar::ProgressBar;
use crate::ui::recent_log_display::RecentLogDisplay;
use crate::ui::screen::line_printer::LinePrinter;
use crate::ui::screen::screen::{Dimensions, Screen};
use crate::ui::ui_component::UiComponent;
use crossbeam_channel::Receiver;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

struct TtyPrinter<S: Screen> {
    _fan: Fan<'static, Event>,
    printer: LinePrinter<S>,
    title: Line,
    progress_bar: ProgressBar,
    recent_log_display: RecentLogDisplay,
}

/// Renders the given components into a list of `Line` sized within the given `width` and `height`.
fn render_screen(components: &[&dyn UiComponent], dimensions: Dimensions) -> Vec<Line> {
    let height = dimensions.height;
    let width = dimensions.width;
    let mut lines = Vec::<Line>::new();

    for (component, n_left) in components.into_iter().zip((0..components.len()).rev()) {
        if lines.len() >= height {
            break;
        }

        let left = height - lines.len();
        let desired_height = left.checked_sub(n_left).unwrap_or(1);

        lines.extend(component.render(width, desired_height));
    }

    lines
}

impl<S: Screen> TtyPrinter<S> {
    fn print_frame(&mut self) {
        let dims = self.printer.dimensions();

        let lines = render_screen(
            &[
                &self.title.center(dims.width).truncate(dims.width),
                &self.progress_bar,
                &self.recent_log_display,
            ],
            dims,
        );

        for (i, line) in lines.into_iter().enumerate() {
            self.printer.print_line_at(line, i);
        }
    }

    fn new(
        event_receiver: Receiver<Event>,
        title: Line,
        initial_range: NumericRange,
        screen: S,
    ) -> Self {
        let fan = Fan::new(event_receiver);
        let printer = LinePrinter::new(screen);
        let progress_bar = ProgressBar::new(fan.subscribe(), initial_range);
        let recent_log_display = RecentLogDisplay::new(fan.subscribe());

        Self {
            _fan: fan,
            printer,
            title,
            progress_bar,
            recent_log_display,
        }
    }
}

/// Instantiation of this struct renders a full-fledged TUI for TTY interfaces based on the given `Event` stream. Use `NoTtyUi` for a traditional "stream of logs" interface.
///
/// There should be no more than one instance of either `TtyUi` or `NoTtyUi` at any time.
///
/// The TUI stops rendering when this struct is dropped.
pub struct TtyUi {
    clock: PeriodicNotifier,
    frame_loop: Listener<'static, ()>,
}

impl TtyUi {
    /// Displays the `TtyUi` until this struct is dropped.
    ///
    /// Note that it's up to the caller to determine if the given `screen` is valid or not.
    pub fn start<S: Screen + Send + 'static>(
        initial_range: NumericRange,
        title: Line,
        event_receiver: Receiver<Event>,
        screen: S,
    ) -> Self {
        let mut tty_printer = TtyPrinter::new(event_receiver, title, initial_range, screen);
        let clock = PeriodicNotifier::new(Duration::from_millis(500));
        let frame_loop = Listener::spawn(clock.receiver(), move |_| tty_printer.print_frame());

        Self { clock, frame_loop }
    }

    pub fn cancel(&self) {
        self.clock.stop();
        self.frame_loop.stop();
    }
}

impl Drop for TtyUi {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::mailbox::Mailbox;
    use crate::parasect::event::Event::{RangeInvalidated, WorkerMessageSent};
    use crate::parasect::types::ParasectPayloadAnswer::{Bad, Good};
    use crate::parasect::types::ParasectPayloadResult::Continue;
    use crate::parasect::worker::PointCompletionMessageType::{Completed, Started};
    use crate::parasect::worker::WorkerMessage;
    use crate::test_util::test_util::test_util::{ib, r, wait_for_condition};
    use crate::ui::line::mkline;
    use crate::ui::screen::test_screen::TestScreen;
    use crate::ui::segment::{Attributes, Color};
    use crossbeam_channel::unbounded;
    use std::sync::Mutex;

    #[test]
    fn test_render_screen_1() {
        let c1 = mkline!("sussus amogus");
        let c2 = [mkline!(("bar", Color::Green)), mkline!("foo")];

        let rendered = render_screen(&[&c1, &c2.as_slice()], (2, 3).into());
        let expected = [mkline!("su…"), mkline!(("bar", Color::Green))];

        assert_eq!(expected.into_iter().collect_vec(), rendered);
    }

    #[test]
    fn test_render_screen_2() {
        let c1 = [mkline!("a"), mkline!("b"), mkline!("c"), mkline!("d")];
        let c2 = [mkline!("e"), mkline!("f"), mkline!("g")];
        let c3 = [mkline!("h"), mkline!("i")];

        let rendered = render_screen(
            &[&c1.as_slice(), &c2.as_slice(), &c3.as_slice()],
            (7, 3).into(),
        );
        let expected = [
            mkline!("a"),
            mkline!("b"),
            mkline!("c"),
            mkline!("d"),
            mkline!("e"),
            mkline!("f"),
            mkline!("h"),
        ];

        assert_eq!(expected.into_iter().collect_vec(), rendered);
    }

    #[test]
    fn test_print_frame_start() {
        let (send, recv) = unbounded();
        let screen = Arc::new(Mutex::new(TestScreen::new((8, 10))));

        let mut tty_printer = TtyPrinter::new(
            recv,
            mkline!(("foo", Color::Blue)),
            r(0, 40),
            screen.clone(),
        );

        tty_printer.print_frame();

        assert_eq!(
            screen.lock().unwrap().state(),
            vec![
                mkline!("   ", ("foo", Color::Blue), "    "),
                mkline!(("██████████", Color::Blue)),
                mkline!(("██████████", Color::Blue)),
                mkline!("^        ^"),
                mkline!("0       40"),
                Line::blank(10),
                Line::blank(10),
                Line::blank(10),
            ]
            .into_boxed_slice()
        );
    }

    #[test]
    fn test_print_frame_rerender() {
        let (send, recv) = unbounded();
        let screen = Arc::new(Mutex::new(TestScreen::new((8, 10))));

        let mut tty_printer = TtyPrinter::new(
            recv,
            mkline!(("foo", Color::Blue)),
            r(0, 40),
            screen.clone(),
        );

        tty_printer.print_frame();
        tty_printer.print_frame();

        assert_eq!(
            screen.lock().unwrap().state(),
            vec![
                mkline!("   ", ("foo", Color::Blue), "    "),
                mkline!(("██████████", Color::Blue)),
                mkline!(("██████████", Color::Blue)),
                mkline!("^        ^"),
                mkline!("0       40"),
                Line::blank(10),
                Line::blank(10),
                Line::blank(10),
            ]
            .into_boxed_slice()
        );
    }

    #[test]
    fn test_print_frame_render_new_frame() {
        let (send, recv) = unbounded();
        let screen = Arc::new(Mutex::new(TestScreen::new((8, 10))));

        let mut tty_printer = TtyPrinter::new(
            recv.clone(),
            mkline!(("foo", Color::Blue)),
            r(0, 40),
            screen.clone(),
        );

        tty_printer.print_frame();

        send.send_msg(WorkerMessageSent(WorkerMessage {
            thread_id: 0,
            left: r(0, 19),
            point: ib(20),
            right: r(21, 40),
            msg_type: Started,
        }));

        send.send_msg(WorkerMessageSent(WorkerMessage {
            thread_id: 0,
            left: r(0, 19),
            point: ib(20),
            right: r(21, 40),
            msg_type: Completed(Continue(Bad)),
        }));

        send.send_msg(RangeInvalidated(r(20, 40), Bad));

        send.send_msg(WorkerMessageSent(WorkerMessage {
            thread_id: 0,
            left: r(0, 9),
            point: ib(10),
            right: r(11, 20),
            msg_type: Started,
        }));

        send.send_msg(WorkerMessageSent(WorkerMessage {
            thread_id: 1,
            left: r(11, 14),
            point: ib(15),
            right: r(16, 20),
            msg_type: Started,
        }));

        send.send_msg(WorkerMessageSent(WorkerMessage {
            thread_id: 1,
            left: r(11, 14),
            point: ib(15),
            right: r(16, 20),
            msg_type: Completed(Continue(Good)),
        }));

        send.send_msg(RangeInvalidated(r(11, 15), Good));

        wait_for_condition(
            || recv.is_empty(),
            Duration::from_secs(3),
            "Receiver was never drained",
        );

        // give the internals time to process the messages
        thread::sleep(Duration::from_millis(200));

        tty_printer.print_frame();

        assert_eq!(
            screen.lock().unwrap().state(),
            vec![
                mkline!("   ", ("foo", Color::Blue), "    "),
                mkline!(
                    ("█████", Color::Blue),
                    ("█", Color::Yellow, Attributes::Blink),
                    ("██", Color::Green),
                    ("██", Color::Blue)
                ),
                mkline!(
                    ("█████", Color::Blue),
                    ("█", Color::Yellow, Attributes::Blink),
                    ("██", Color::Green),
                    ("██", Color::Blue)
                ),
                mkline!("^        ^"),
                mkline!("0       19"),
                mkline!("[", ("11", Color::Blue), ", ", ("15", Color::Blue), "]:…"),
                mkline!(
                    "1: ",
                    ("15", Color::Blue, Attributes::Bold),
                    " ",
                    ("Good", Color::Green, Attributes::Bold)
                ),
                mkline!("0: ", ("10", Color::Blue, Attributes::Bold), "     "),
            ]
            .into_boxed_slice()
        );
    }
}
