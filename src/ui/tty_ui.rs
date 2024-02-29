use crate::collections::collect_collection::CollectVec;
use crate::messaging::fan::Fan;
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

/// Instantiation of this struct renders a full-fledged TUI for TTY interfaces based on the given `Event` stream. Use `NoTtyUi` for a traditional "stream of logs" interface.
///
/// There should be no more than one instance of either `TtyUi` or `NoTtyUi` at any time.
///
/// The TUI stops rendering when this struct is dropped.
pub struct TtyUi {
    cancel: Arc<AtomicBool>,
    _fan: Fan<'static, Event>,
}

impl TtyUi {
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

    pub fn start<S: Screen + Send + 'static>(
        initial_range: NumericRange,
        title: Line,
        event_receiver: Receiver<Event>,
        screen: S,
    ) -> Option<Self> {
        if termion::terminal_size().is_err() {
            return None;
        }

        let fan = Fan::new(event_receiver);
        let cancel = Arc::new(AtomicBool::new(false));

        let progress_bar = ProgressBar::new(fan.subscribe(), initial_range);
        let recent_log_display = RecentLogDisplay::new(fan.subscribe());
        let cancel_clone = cancel.clone();

        thread::spawn(move || {
            let mut printer = LinePrinter::new(screen);

            while !cancel_clone.load(Ordering::Relaxed) {
                let dims = printer.dimensions();

                let lines = Self::render_screen(
                    &[
                        &title.center(dims.width).truncate(dims.width),
                        &progress_bar,
                        &recent_log_display,
                    ],
                    dims,
                );

                for (i, line) in lines.into_iter().enumerate() {
                    printer.print_line_at(line, i);
                }

                thread::sleep(Duration::from_millis(500));
            }
        });

        Some(Self { cancel, _fan: fan })
    }

    pub fn cancel(&self) {
        print!("{}", termion::cursor::Restore);
        self.cancel.store(false, Ordering::Relaxed);
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
    use crate::ui::line::mkline;
    use crate::ui::segment::Color;

    #[test]
    fn test_render_screen_1() {
        let c1 = mkline!("sussus amogus");
        let c2 = [mkline!(("bar", Color::Green)), mkline!("foo")];

        let rendered = TtyUi::render_screen(&[&c1, &c2.as_slice()], (2, 3).into());
        let expected = [mkline!("su…"), mkline!(("bar", Color::Green))];

        assert_eq!(expected.into_iter().collect_vec(), rendered);
    }

    #[test]
    fn test_render_screen_2() {
        let c1 = [mkline!("a"), mkline!("b"), mkline!("c"), mkline!("d")];
        let c2 = [mkline!("e"), mkline!("f"), mkline!("g")];
        let c3 = [mkline!("h"), mkline!("i")];

        let rendered = TtyUi::render_screen(
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
}
