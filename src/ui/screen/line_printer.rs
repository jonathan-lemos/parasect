use crate::ui::line::Line;
use crate::ui::screen::screen::{Dimensions, Screen};
use std::collections::HashMap;

/// Prints lines to the given screen.
///
/// Caches lines that are equal to what's currently on screen. Also truncates any lines that would have overflown.
pub struct LinePrinter<S: Screen> {
    screen: S,
    old_dimensions: Dimensions,
    line_cache: HashMap<usize, Line>,
}

impl<S: Screen> LinePrinter<S> {
    pub fn new(screen: S) -> Self {
        let old_dimensions = screen.dimensions();
        Self {
            screen,
            line_cache: HashMap::new(),
            old_dimensions,
        }
    }

    /// Gets the dimensions of the underlying screen.
    pub fn dimensions(&self) -> Dimensions {
        self.screen.dimensions()
    }

    /// Prints the given line at the given index.
    ///
    /// If the line is longer than the underlying screen's width, truncates the line to fit.
    /// If the line's row is too great to fit on the screen, this function does nothing.
    pub fn print_line_at(&mut self, line: Line, row: usize) {
        let dims = self.dimensions();

        if row >= dims.height {
            return;
        }

        if self.old_dimensions != dims {
            self.old_dimensions = dims;
            self.line_cache = HashMap::new();
            self.screen.reset();
        }

        if self.line_cache.get(&row) == Some(&line) {
            return;
        }

        let line = line.truncate(dims.width).pad(dims.width);

        self.screen.print_line_at(&line, row);
        self.line_cache.insert(row, line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::line::mkline;
    use crate::ui::screen::test_screen::TestScreen;

    #[test]
    fn test_dimensions() {
        let lp = LinePrinter::new(TestScreen::new((3, 4)));

        assert_eq!(lp.dimensions(), (3, 4).into());
    }

    #[test]
    fn test_print_line_at() {
        let l1 = mkline!("sus");
        let l2 = mkline!("amogus");

        let mut lp = LinePrinter::new(TestScreen::new((2, 10)));

        lp.print_line_at(l1, 0);
        lp.print_line_at(l2, 1);

        assert_eq!(
            lp.screen.state(),
            vec![mkline!("sus").pad(10), mkline!("amogus").pad(10)].into()
        )
    }

    #[test]
    fn test_print_line_at_overwrites() {
        let l1 = mkline!("amogus");
        let l2 = mkline!("sus");

        let mut lp = LinePrinter::new(TestScreen::new((2, 10)));

        lp.print_line_at(l1, 0);
        lp.print_line_at(l2, 0);

        assert_eq!(
            lp.screen.state(),
            vec![mkline!("sus").pad(10), Line::blank(10)].into()
        )
    }

    #[test]
    fn test_print_line_at_truncates() {
        let l1 = mkline!("amogus");
        let l2 = mkline!("sus");

        let mut lp = LinePrinter::new(TestScreen::new((2, 5)));

        lp.print_line_at(l1, 0);
        lp.print_line_at(l2, 1);

        assert_eq!(
            lp.screen.state(),
            vec![mkline!("amog…"), mkline!("sus").pad(5)].into()
        )
    }

    #[test]
    fn test_print_line_cache() {
        let l1 = mkline!("amogus");

        let mut lp = LinePrinter::new(TestScreen::new((2, 6)));

        lp.print_line_at(l1.clone(), 0);
        lp.print_line_at(l1.clone(), 1);
        lp.print_line_at(l1.clone(), 1);

        assert_eq!(
            lp.screen.state(),
            vec![mkline!("amogus"), mkline!("amogus")].into()
        )
    }

    #[test]
    fn test_print_line_cache_resize() {
        let l1 = mkline!("amogus");

        let mut lp = LinePrinter::new(TestScreen::new((2, 6)));

        lp.print_line_at(l1.clone(), 0);
        lp.print_line_at(l1.clone(), 1);
        lp.screen.resize((2, 8).into());
        lp.print_line_at(l1.clone(), 1);

        assert_eq!(
            lp.screen.state(),
            vec![Line::blank(8), mkline!("amogus").pad(8)].into()
        )
    }
}
