use crate::collections::collect_collection::CollectVec;
use crate::ui::line::Line;
use crate::ui::screen::screen::{Dimensions, Screen};
use crate::ui::segment::{Attributes, Color, Segment};
use std::collections::VecDeque;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Hash)]
struct CellState {
    pub char: Option<char>,
    pub color: Color,
    pub attributes: Attributes,
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
    (0..length)
        .map(|_| CellState {
            char: None,
            color: Color::Default,
            attributes: Attributes::empty(),
        })
        .collect()
}

impl TestScreen {
    pub fn new(dimensions: Dimensions) -> Self {
        Self {
            dimensions,
            cursor_pos: 0,
            screen_state: blank_state(dimensions.size()),
        }
    }

    fn append_char(&mut self, char: char, color: Color, attributes: Attributes) {
        let cell_ptr = self.screen_state.get_mut(self.cursor_pos).unwrap();
        *cell_ptr = CellState {
            char: Some(char),
            color,
            attributes,
        };

        if self.cursor_pos + 1 < self.dimensions.size() {
            self.cursor_pos += 1;
        } else {
            self.screen_state.pop_front();
        }
    }

    /// Resizes the `TestScreen`.
    ///
    /// If the contents of the screen cannot fit in the new dimensions, the last *height \* width* characters will be kept.
    pub fn resize(&mut self, dimensions: Dimensions) {
        let old_dimensions = self.dimensions;
        self.dimensions = dimensions;

        if dimensions.size() >= old_dimensions.size() {
            return;
        }

        let delta = old_dimensions.size() - dimensions.size();
        self.screen_state = self.screen_state.iter().skip(delta).map(|x| *x).collect()
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
                        cell.char.unwrap_or(' ').to_string(),
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
            for char in seg.content().chars() {
                self.append_char(char, seg.color(), seg.attributes());
            }
        }
    }

    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        let pos = self.dimensions.coord_to_pos((row, col));
        assert!(pos < self.dimensions.size());
        self.cursor_pos = pos;
    }

    fn reset(&mut self) {
        self.screen_state = blank_state(self.dimensions.size());
        self.cursor_pos = 0;
    }
}
