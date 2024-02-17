use crate::parasect::event::Event;
use crate::range::numeric_range::NumericRange;
use crate::ui::no_tty_ui::NoTtyUi;
use crate::ui::tty_ui::TtyUi;
use crate::ui::ui::Ui::*;
use crossbeam_channel::Receiver;

pub enum Ui {
    Tty(TtyUi),
    NoTty(NoTtyUi),
}

impl Ui {
    /*
    pub fn start(initial_range: NumericRange, event_receiver: Receiver<Event>) -> Self {
        TtyUi::start(initial_range.clone(), event_receiver.clone())
            .map(Tty)
            .unwrap_or_else(|| NoTty(NoTtyUi::start(initial_range, event_receiver)))
    }
    */
}
