use crate::parasect::event::Event;
use crate::range::numeric_range::NumericRange;
use crate::ui::line::Line;
use crate::ui::no_tty_ui::NoTtyUi;
use crate::ui::screen::terminal_screen::TerminalScreen;
use crate::ui::tty_ui::TtyUi;
use crate::ui::ui::Ui::*;
use crossbeam_channel::Receiver;

pub enum Ui {
    Tty(TtyUi),
    NoTty(NoTtyUi),
}

impl Ui {
    pub fn start(
        initial_range: NumericRange,
        title: Line,
        event_receiver: Receiver<Event>,
        no_tty: bool,
    ) -> Self {
        if no_tty {
            NoTty(NoTtyUi::start(
                initial_range,
                title.plaintext(),
                event_receiver,
            ))
        } else {
            TtyUi::start(
                initial_range.clone(),
                title.clone(),
                event_receiver.clone(),
                TerminalScreen::new(),
            )
            .map(Tty)
            .unwrap_or_else(|| {
                NoTty(NoTtyUi::start(
                    initial_range,
                    title.plaintext(),
                    event_receiver,
                ))
            })
        }
    }
}
