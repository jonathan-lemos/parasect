use crate::collections::collect_collection::CollectVec;
use crate::parasect::event::Event;
use crate::range::numeric_range::NumericRange;
use crate::threading::fan::Fan;
use crate::ui::line::Line;
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
use termion::is_tty;

/// Instantiation of this struct renders a full-fledged TUI for TTY interfaces based on the given `Event` stream. Use `NoTtyUi` for a traditional "stream of logs" interface.
///
/// There should be one and only one instance of either `TtyUi` or `NoTtyUi` at any time.
///
/// The TUI stops rendering when this struct is dropped.
pub struct TtyUi {
    cancel: Arc<AtomicBool>,
    line_cache: Arc<DashMap<usize, Line>>,
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

        for (component, n_left) in components.into_iter().zip((components.len() - 1)..0) {
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
        for (line_option, idx) in lines.iter().zip(1..) {
            print!("{}", termion::cursor::Goto(idx, 1));
            unwrap_or!(line_option, continue).print();
        }
    }

    pub fn start(
        initial_range: NumericRange,
        title: &str,
        event_receiver: Receiver<Event>,
    ) -> Option<Self> {
        if Self::get_bounds().is_none() {
            return None;
        }

        let fan = Fan::new(event_receiver);
        let cancel = Arc::new(AtomicBool::new(false));
        let line_cache = Arc::new(DashMap::new());

        print!("{}", termion::cursor::Hide);
        Self::clear_screen();

        let progress_bar = ProgressBar::new(fan.subscribe().clone(), initial_range);
        let recent_log_display = RecentLogDisplay::new(fan.subscribe().clone());
        let cancel_clone = cancel.clone();
        let title_clone = title.to_string();
        let line_cache_clone = line_cache.clone();

        thread::spawn(move || {
            while !cancel_clone.load(Ordering::Relaxed) {
                let (width, height) = unwrap_or!(Self::get_bounds(), continue);

                let screen = Self::render_screen(
                    &[
                        &Line::from(title_clone.as_str()).center(width),
                        &progress_bar,
                        &recent_log_display,
                    ],
                    width,
                    height,
                );

                let screen_deduped = Self::remove_cached(screen, &line_cache_clone);

                Self::print_deduped_lines(&screen_deduped);
            }
        });

        Some(Self {
            cancel,
            line_cache,
            _fan: fan,
        })
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
