use crate::ui::line::Line;
use crate::ui::screen::screen::{Dimensions, Screen};
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;

static INSTANTIATED: Mutex<bool> = Mutex::new(false);

pub struct TerminalScreen {}

impl TerminalScreen {
    /// Creates a new `TerminalScreen`.
    ///
    /// Instantiation clears the screen and hides the cursor.
    pub fn new() -> Self {
        let mut instantiated_lock = INSTANTIATED.lock().unwrap();
        assert_eq!(
            *instantiated_lock.deref(),
            false,
            "Cannot instantiate more than one TerminalScreen at once."
        );
        *instantiated_lock.deref_mut() = true;

        print!(
            "{}{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1),
            termion::cursor::Hide
        );
        TerminalScreen {}
    }

    pub fn output_is_tty() -> bool {
        return termion::terminal_size().is_ok();
    }
}

impl Drop for TerminalScreen {
    fn drop(&mut self) {
        print!("{}", termion::cursor::Show);
        println!();
        let mut instantiated_lock = match INSTANTIATED.lock() {
            Ok(l) => l,
            // if the mutex is poisoned, there's no point in changing its value
            Err(_) => return,
        };
        *instantiated_lock.deref_mut() = false;
    }
}

impl Screen for TerminalScreen {
    fn append_line(&mut self, line: &Line) {
        line.print()
    }

    fn dimensions(&self) -> Dimensions {
        termion::terminal_size()
            .map(|(x, y)| (y as usize, x as usize))
            .unwrap_or((0, 0))
            .into()
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        print!(
            "{}",
            termion::cursor::Goto((col as u16) + 1, (row as u16) + 1)
        );
    }

    fn newline(&mut self) {
        println!()
    }

    fn reset(&mut self) {
        print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_multiple_instantiation_panics() {
        let _t1 = TerminalScreen::new();
        TerminalScreen::new();
    }
}
