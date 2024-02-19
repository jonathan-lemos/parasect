use crate::ui::line::Line;
use crate::ui::screen::screen::{Dimensions, Screen};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub struct TerminalScreen {}

static INSTANTIATED: Mutex<bool> = Mutex::new(false);

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

        println!(
            "{}{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1),
            termion::cursor::Hide
        );
        TerminalScreen {}
    }
}

impl Drop for TerminalScreen {
    fn drop(&mut self) {
        println!("{}", termion::cursor::Restore);
        let mut instantiated_lock = INSTANTIATED.lock().unwrap();
        *instantiated_lock.deref_mut() = false;
    }
}

impl Screen for TerminalScreen {
    fn append_line(&mut self, line: &Line) {
        line.print()
    }

    fn dimensions(&self) -> Dimensions {
        termion::terminal_size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap_or((0, 0))
            .into()
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        println!(
            "{}",
            termion::cursor::Goto((row as u16) + 1, (col as u16) + 1)
        );
    }

    fn reset(&mut self) {
        print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
    }
}
