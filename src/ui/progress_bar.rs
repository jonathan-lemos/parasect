use crate::range::numeric_range::NumericRange;
use crate::range::numeric_range_set::NumericRangeSet;
use crate::threading::background_loop::BackgroundLoop;
use crate::threading::background_loop::BackgroundLoopBehavior::DontCancel;
use crate::ui::line::Line;
use crate::ui::ui_component::UiComponent;
use crate::unwrap_or;
use crossbeam_channel::Receiver;
use std::sync::{Arc, RwLock};

pub struct ProgressBar {
    receiver_listener: BackgroundLoop,
    valid_ranges: Arc<RwLock<NumericRangeSet>>,
}

impl ProgressBar {
    pub fn new(
        initial_range: NumericRange,
        invalidated_range_receiver: Receiver<NumericRange>,
    ) -> Self {
        let mut range_set = NumericRangeSet::new();
        range_set.add(initial_range);

        let valid_ranges = Arc::new(RwLock::new(range_set));

        let valid_ranges_clone = valid_ranges.clone();

        Self {
            receiver_listener: BackgroundLoop::spawn(invalidated_range_receiver, move |range| {
                valid_ranges_clone.write().unwrap().remove(&range);
                DontCancel
            }),
            valid_ranges,
        }
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        todo!()
    }
}

impl UiComponent for ProgressBar {
    fn render(&self, width: usize) -> Vec<Line> {
        let valid_ranges = self.valid_ranges.read().unwrap();

        let (low, high) = unwrap_or!(valid_ranges.bounds(), return Vec::new());

        todo!()
    }
}
