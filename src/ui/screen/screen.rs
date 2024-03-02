use crate::ui::line::Line;
use std::sync::{Arc, Mutex};

/// Represents a size in 2-dimensional space.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Copy, Clone)]
pub struct Dimensions {
    pub height: usize,
    pub width: usize,
}

impl Dimensions {
    pub fn new(height: usize, width: usize) -> Self {
        Self { height, width }
    }

    /// Turns a (row, column) 0-indexed coordinate into a scalar position.
    ///
    /// `(coord_to_pos . pos_to_coord) == id`, if the given coord is in range.
    ///
    /// No checking is done to ensure that the returned position is within the dimensions.
    pub fn coord_to_pos(&self, coord: (usize, usize)) -> usize {
        coord.0 * self.width + coord.1
    }

    /// Turns a scalar quantity (0-indexed) into a coordinate, if in range.
    ///
    /// The coordinate is (row, column) and 0-indexed.
    pub fn pos_to_coord(&self, pos: usize) -> Option<(usize, usize)> {
        let row = pos / self.width;
        let col = pos % self.width;

        if row >= self.height {
            None
        } else {
            Some((row, col))
        }
    }

    /// Returns the amount of cells in the area represented by the `Dimensions`.
    pub fn size(&self) -> usize {
        self.height * self.width
    }
}

impl From<(usize, usize)> for Dimensions {
    fn from(value: (usize, usize)) -> Self {
        Self::new(value.0, value.1)
    }
}

pub trait Screen {
    /// Prints a line like `print!()` would. Use `append_line_with_newline()` to include a newline.
    fn append_line(&mut self, line: &Line);

    /// Gets the dimensions (in characters) of the screen.
    fn dimensions(&self) -> Dimensions;

    /// Moves the cursor to the given coordinate (both 0-indexed).
    fn move_cursor(&mut self, row: usize, col: usize);

    /// Moves the cursor to the start of the next line.
    fn newline(&mut self);

    /// Prints a line starting from the first character in the given row (0-indexed).
    ///
    /// Also moves the cursor to the end of the written line.
    fn print_line_at(&mut self, line: &Line, row: usize) {
        self.move_cursor(row, 0);
        self.append_line(line);
    }

    /// Clears the screen, and resets the cursor to (0, 0).
    fn reset(&mut self);
}

impl<S: Screen> Screen for Arc<Mutex<S>> {
    fn append_line(&mut self, line: &Line) {
        self.lock().unwrap().append_line(line)
    }

    fn dimensions(&self) -> Dimensions {
        self.lock().unwrap().dimensions()
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        self.lock().unwrap().move_cursor(row, col)
    }

    fn newline(&mut self) {
        self.lock().unwrap().newline()
    }

    fn reset(&mut self) {
        self.lock().unwrap().reset()
    }
}
