use crate::collections::collect_collection::CollectVec;
use crate::ui::line::Line;
use crate::ui::screen::screen::{Dimensions, Screen};
use crate::ui::segment::{Attributes, Color, Segment};
use crate::util::macros::unwrap_or;
use std::cmp::min;
use std::collections::VecDeque;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash)]
struct CellState {
    // String because of bullshit where a letter may be composed of more than one char
    pub char: Option<String>,
    pub color: Color,
    pub attributes: Attributes,
}

impl CellState {
    fn blank() -> Self {
        Self {
            char: None,
            color: Color::Default,
            attributes: Attributes::empty(),
        }
    }
}

/// A pseudo-screen that writes to internal memory.
///
/// Meant for use with tests that need to verify what's on the screen.
pub struct TestScreen {
    dimensions: Dimensions,
    cursor_pos: usize,
    screen_state: VecDeque<CellState>,
}

fn blank_state(length: usize) -> VecDeque<CellState> {
    (0..length).map(|_| CellState::blank()).collect()
}

impl TestScreen {
    pub fn new<I: Into<Dimensions>>(dimensions: I) -> Self {
        let d = dimensions.into();
        Self {
            dimensions: d,
            cursor_pos: 0,
            screen_state: blank_state(d.size()),
        }
    }

    fn append_char(&mut self, char: &str, color: Color, attributes: Attributes) {
        let cell_ptr = if self.cursor_pos < self.screen_state.len() {
            self.screen_state.get_mut(self.cursor_pos).unwrap()
        } else {
            self.screen_state.pop_front();
            self.screen_state.push_back(CellState::blank());
            self.screen_state.back_mut().unwrap()
        };

        *cell_ptr = CellState {
            char: Some(char.to_string()),
            color,
            attributes,
        };

        if self.cursor_pos < self.dimensions.size() {
            self.cursor_pos += 1;
        }
    }

    /// Resizes the `TestScreen`.
    ///
    /// If the contents of the screen cannot fit in the new dimensions, the last *height \* width* characters will be kept.
    pub fn resize(&mut self, dimensions: Dimensions) {
        let old_dimensions = self.dimensions;
        self.dimensions = dimensions;

        if dimensions.size() >= old_dimensions.size() {
            let delta = dimensions.size() - old_dimensions.size();
            for _ in 0..delta {
                self.screen_state.push_back(CellState::blank());
            }
            return;
        }

        let delta = old_dimensions.size() - dimensions.size();
        self.screen_state = self
            .screen_state
            .iter()
            .skip(delta)
            .map(|x| x.clone())
            .collect()
    }

    /// Gets a list of lines corresponding to what the screen currently looks like.
    pub fn state(&self) -> Box<[Line]> {
        (0..self.dimensions.height)
            .map(|i| {
                Line::from_iter((0..self.dimensions.width).map(|j| {
                    let pos = self.dimensions.coord_to_pos((i, j));
                    assert!(pos < self.screen_state.len());

                    let cell = self.screen_state.get(pos).unwrap();

                    Segment::new(
                        cell.char
                            .as_ref()
                            .map(|x| x.as_str())
                            .unwrap_or(" ")
                            .to_string(),
                        cell.color,
                        cell.attributes,
                    )
                }))
            })
            .collect_vec()
            .into_boxed_slice()
    }
}

impl Screen for TestScreen {
    fn append_line(&mut self, line: &Line) {
        for seg in line.iter() {
            for char in seg.content().graphemes(true) {
                self.append_char(char, seg.color(), seg.attributes());
            }
        }
    }

    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        let pos = self.dimensions.coord_to_pos((row, col));
        self.cursor_pos = min(pos, self.dimensions.size());
    }

    fn newline(&mut self) {
        // if we are out of bounds, no need to do anything
        let (row, col) = unwrap_or!(self.dimensions.pos_to_coord(self.cursor_pos), return);

        if col == 0 {
            // already at the start of a new line. no need to newline again
            return;
        }

        if row + 1 >= self.dimensions.height {
            for _ in 0..self.dimensions.width {
                self.screen_state.pop_front();
            }

            for _ in 0..self.dimensions.width {
                self.screen_state.push_back(CellState::blank());
            }
            self.cursor_pos = self.dimensions.coord_to_pos((row, 0));
        } else {
            self.cursor_pos = self.dimensions.coord_to_pos((row + 1, 0));
        }
    }

    fn reset(&mut self) {
        self.screen_state = blank_state(self.dimensions.size());
        self.cursor_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::line::mkline;

    fn assert_invariants(screen: &TestScreen) {
        assert_eq!(
            screen.dimensions.size(),
            screen.screen_state.len(),
            "The screen dimensions must match the length of the screen state."
        );
    }

    #[test]
    fn test_append_line_basic() {
        let mut ts = TestScreen::new((2, 10));
        ts.append_line(&mkline!("foo"));

        assert_eq!(
            ts.state(),
            vec![mkline!("foo").pad(10), Line::blank(10)].into()
        );

        assert_invariants(&ts);
    }

    #[test]
    fn test_append_line_multiple() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("foo"));
        ts.append_line(&mkline!("bar"));

        assert_eq!(
            ts.state(),
            vec![mkline!("fooba"), mkline!("r").pad(5)].into()
        );

        assert_invariants(&ts);
    }

    #[test]
    fn test_append_line_exactly_at_end() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("foo"));
        ts.append_line(&mkline!("ba"));

        assert_eq!(ts.state(), vec![mkline!("fooba"), Line::blank(5)].into());

        assert_invariants(&ts);
    }

    #[test]
    fn test_append_line_overflow() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("foobar"));
        ts.append_line(&mkline!("amogus"));

        assert_eq!(ts.state(), vec![mkline!("obara"), mkline!("mogus")].into());
        assert_invariants(&ts);
    }

    #[test]
    fn test_append_line_single_line_overflow() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("foobaramogus"));

        assert_eq!(ts.state(), vec![mkline!("obara"), mkline!("mogus")].into());
        assert_invariants(&ts);
    }

    #[test]
    fn test_append_line_snug() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("abcde"));
        ts.append_line(&mkline!("fghij"));

        assert_eq!(ts.state(), vec![mkline!("abcde"), mkline!("fghij")].into());
        assert_invariants(&ts);
    }

    #[test]
    fn test_append_line_snug_single() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("abcdefghij"));

        assert_eq!(ts.state(), vec![mkline!("abcde"), mkline!("fghij")].into());
        assert_invariants(&ts);
    }

    #[test]
    fn test_append_line_when_full() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("foobaramogus"));
        ts.append_line(&mkline!("sus"));

        assert_eq!(ts.state(), vec![mkline!("ramog"), mkline!("ussus")].into());
        assert_invariants(&ts);
    }

    #[test]
    fn test_dimensions() {
        let ts = TestScreen::new((15, 10));
        assert_eq!(ts.dimensions(), (15, 10).into());
        assert_invariants(&ts);
    }

    #[test]
    fn test_move_cursor() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("mogus"));
        ts.move_cursor(0, 1);
        ts.append_line(&mkline!("sus"));

        assert_eq!(ts.state(), vec![mkline!("msuss"), Line::blank(5)].into());
        assert_invariants(&ts);
    }

    #[test]
    fn test_move_cursor_2() {
        let mut ts = TestScreen::new((2, 6));
        ts.append_line(&mkline!("sussus"));
        ts.append_line(&mkline!("amogus"));
        ts.move_cursor(0, 2);
        ts.append_line(&mkline!("abcdef"));

        assert_eq!(
            ts.state(),
            vec![mkline!("suabcd"), mkline!("efogus")].into()
        );
        assert_invariants(&ts);
    }

    #[test]
    fn test_newline() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("foo"));
        ts.newline();
        ts.append_line(&mkline!("bar"));

        assert_eq!(
            ts.state(),
            vec![mkline!("foo").pad(5), mkline!("bar").pad(5)].into()
        );
        assert_invariants(&ts);
    }

    #[test]
    fn test_newline_not_at_eol() {
        let mut ts = TestScreen::new((3, 5));
        ts.append_line(&mkline!("abcde"));
        ts.newline();
        ts.append_line(&mkline!("fgh"));

        assert_eq!(
            ts.state(),
            vec![
                mkline!("abcde").pad(5),
                mkline!("fgh").pad(5),
                Line::blank(5),
            ]
            .into()
        );
        assert_invariants(&ts);
    }

    #[test]
    fn test_newline_overflow() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("abcde"));
        ts.append_line(&mkline!("fgh"));
        ts.newline();
        ts.append_line(&mkline!("ijk"));

        assert_eq!(
            ts.state(),
            vec![mkline!("fgh").pad(5), mkline!("ijk").pad(5)].into()
        );
        assert_invariants(&ts);
    }

    #[test]
    fn test_print_line_at() {
        let mut ts = TestScreen::new((2, 5));

        ts.print_line_at(&mkline!("gus"), 1);
        ts.print_line_at(&mkline!("amo"), 0);

        assert_eq!(
            ts.state(),
            vec![mkline!("amo").pad(5), mkline!("gus").pad(5)].into()
        );

        assert_invariants(&ts);
    }

    #[test]
    fn test_reset() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("sussymogus"));
        ts.reset();
        ts.append_line(&mkline!("abc"));

        assert_eq!(
            ts.state(),
            vec![mkline!("abc").pad(5), Line::blank(5)].into()
        );

        assert_invariants(&ts);
    }

    #[test]
    fn test_resize() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("abcdefghi"));

        ts.resize((3, 4).into());

        assert_invariants(&ts);

        assert_eq!(
            ts.state(),
            vec![mkline!("abcd"), mkline!("efgh"), mkline!("i").pad(4)].into()
        );
    }

    #[test]
    fn test_resize_overflow() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("abcdefghij"));

        ts.resize((2, 3).into());

        assert_eq!(ts.state(), vec![mkline!("efg"), mkline!("hij")].into());

        assert_invariants(&ts);
    }

    #[test]
    fn test_resize_snug() {
        let mut ts = TestScreen::new((3, 4));
        ts.append_line(&mkline!("0123456789ab"));

        ts.resize((2, 6).into());

        assert_eq!(
            ts.state(),
            vec![mkline!("012345"), mkline!("6789ab")].into()
        );

        assert_invariants(&ts);
    }

    #[test]
    fn test_resize_spaces_at_end() {
        let mut ts = TestScreen::new((2, 5));
        ts.append_line(&mkline!("12345678"));

        ts.resize((2, 3).into());

        assert_eq!(ts.state(), vec![mkline!("567"), mkline!("8  ")].into());

        assert_invariants(&ts);
    }
}
