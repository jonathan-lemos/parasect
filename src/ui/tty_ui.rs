use crate::collections::collect_collection::CollectVec;
use crate::parasect::event::Event;
use crate::range::numeric_range::NumericRange;
use crate::threading::fan::Fan;
use crate::ui::line::{print_lines, Line};
use crate::ui::progress_bar::ProgressBar;
use crate::ui::recent_log_display::RecentLogDisplay;
use crate::ui::ui_component::UiComponent;
use crate::util::macros::unwrap_or;
use crossbeam_channel::Receiver;
use dashmap::DashMap;
use std::io::stdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use termion::is_tty;

/// Instantiation of this struct renders a full-fledged TUI for TTY interfaces based on the given `Event` stream. Use `NoTtyUi` for a traditional "stream of logs" interface.
///
/// There should be no more than one instance of either `TtyUi` or `NoTtyUi` at any time.
///
/// The TUI stops rendering when this struct is dropped.
pub struct TtyUi {
    cancel: Arc<AtomicBool>,
    _fan: Fan<Event>,
}

impl TtyUi {
    fn clear_screen() {
        print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
    }

    /// Gets the bounds of the terminal only if this program's stdout is a TTY.
    fn get_bounds() -> Option<(usize, usize)> {
        if !is_tty(&stdout()) {
            return None;
        }

        match termion::terminal_size() {
            Ok((w, h)) => Some((w as usize, h as usize)),
            Err(_) => None,
        }
    }

    /// Renders the given components into a list of `Line` sized within the given `width` and `height`.
    fn render_screen(components: &[&dyn UiComponent], width: usize, height: usize) -> Vec<Line> {
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

    /// Removes cached entries from the input sequence of Lines. Also updates the cache for lines that do not match the cached value.
    fn remove_cached<I: IntoIterator<Item = Line>>(
        lines: I,
        cache: &DashMap<usize, Line>,
    ) -> Vec<Option<Line>> {
        lines
            .into_iter()
            .zip(1..)
            .map(|(line, idx)| {
                if cache.get(&idx).as_deref() == Some(&line) {
                    None
                } else {
                    cache.insert(idx, line.clone());
                    Some(line)
                }
            })
            .collect_vec()
    }

    /// Prints only the lines that are Some.
    fn print_deduped_lines(lines: &Vec<Option<Line>>) {
        let empty = Line::empty();
        print_lines(lines.into_iter().map(|x| x.as_ref().unwrap_or(&empty)));
    }

    pub fn start(
        initial_range: NumericRange,
        title: Line,
        event_receiver: Receiver<Event>,
    ) -> Option<Self> {
        if Self::get_bounds().is_none() {
            return None;
        }

        let fan = Fan::new(event_receiver);
        let cancel = Arc::new(AtomicBool::new(false));

        print!("{}", termion::cursor::Hide);
        Self::clear_screen();

        let progress_bar = ProgressBar::new(fan.subscribe().clone(), initial_range);
        let recent_log_display = RecentLogDisplay::new(fan.subscribe().clone());
        let cancel_clone = cancel.clone();
        let line_cache = Arc::new(DashMap::new());

        thread::spawn(move || {
            while !cancel_clone.load(Ordering::Relaxed) {
                println!("{}", termion::cursor::Goto(1, 1));
                let (width, height) = unwrap_or!(Self::get_bounds(), {
                    thread::sleep(Duration::from_millis(500));
                    continue;
                });

                let screen = Self::render_screen(
                    &[
                        &title.center(width).truncate(width),
                        &progress_bar,
                        &recent_log_display,
                    ],
                    width,
                    height,
                );

                let screen_deduped = Self::remove_cached(screen, &line_cache);

                Self::print_deduped_lines(&screen_deduped);
                thread::sleep(Duration::from_millis(500));
            }
        });

        Some(Self { cancel, _fan: fan })
    }

    pub fn cancel(&self) {
        println!("{}", termion::cursor::Restore);
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
    use crate::collections::collect_collection::CollectHashSet;
    use crate::ui::line::mkline;
    use crate::ui::segment::Color;

    #[test]
    fn test_render_screen_1() {
        let c1 = mkline!("sussus amogus");
        let c2 = [mkline!(("bar", Color::Green)), mkline!("foo")];

        let rendered = TtyUi::render_screen(&[&c1, &c2.as_slice()], 3, 2);
        let expected = [mkline!("su…"), mkline!(("bar", Color::Green))];

        assert_eq!(expected.into_iter().collect_vec(), rendered);
    }

    #[test]
    fn test_render_screen_2() {
        let c1 = [mkline!("a"), mkline!("b"), mkline!("c"), mkline!("d")];
        let c2 = [mkline!("e"), mkline!("f"), mkline!("g")];
        let c3 = [mkline!("h"), mkline!("i")];

        let rendered =
            TtyUi::render_screen(&[&c1.as_slice(), &c2.as_slice(), &c3.as_slice()], 3, 7);
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
    fn test_remove_cached() {
        let cache = [(1, mkline!("foo")), (2, mkline!("bar"))]
            .into_iter()
            .collect();

        let input = [mkline!("foo"), mkline!("q"), mkline!("sus")];

        let removed = TtyUi::remove_cached(input, &cache);

        let expected_removed = [None, Some(mkline!("q")), Some(mkline!("sus"))];
        let expected_new_cache = [(1, mkline!("foo")), (2, mkline!("q")), (3, mkline!("sus"))];

        assert_eq!(expected_removed.into_iter().collect_vec(), removed);
        assert_eq!(
            cache.into_iter().collect_hashset(),
            expected_new_cache.into_iter().collect_hashset()
        );
    }
}
