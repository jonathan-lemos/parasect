use crate::parasect::event::Event;
use crate::parasect::event::Event::*;
use crate::parasect::types::ParasectPayloadAnswer::*;
use crate::parasect::worker::PointCompletionMessageType::*;
use crate::parasect::worker::WorkerMessage;
use crate::range::numeric_range::NumericRange;
use crate::range::numeric_range_set::NumericRangeSet;
use crate::threading::background_loop::BackgroundLoop;
use crate::threading::background_loop::BackgroundLoopBehavior::DontCancel;
use crate::ui::line::Line;
use crate::ui::segment::{Attributes, Color, Segment};
use crate::ui::ui_component::UiComponent;
use crate::unwrap_or;
use crossbeam_channel::Receiver;
use std::sync::{Arc, RwLock};

pub struct ProgressBar {
    receiver_listener: BackgroundLoop,
    good_ranges: Arc<RwLock<NumericRangeSet>>,
    bad_ranges: Arc<RwLock<NumericRangeSet>>,
    valid_ranges: Arc<RwLock<NumericRangeSet>>,
    active: Arc<RwLock<NumericRangeSet>>,
}

impl ProgressBar {
    pub fn new(initial_range: NumericRange, event_receiver: Receiver<Event>) -> Self {
        let good_ranges = Arc::new(RwLock::new(NumericRangeSet::new()));
        let bad_ranges = Arc::new(RwLock::new(NumericRangeSet::new()));
        let active = Arc::new(RwLock::new(NumericRangeSet::new()));
        let valid_ranges = Arc::new(RwLock::new(NumericRangeSet::new()));

        valid_ranges.write().unwrap().add(initial_range);

        let good_ranges_clone = good_ranges.clone();
        let bad_ranges_clone = good_ranges.clone();
        let active_clone = active.clone();
        let valid_ranges_clone = valid_ranges.clone();

        Self {
            good_ranges,
            bad_ranges,
            active,
            valid_ranges,
            receiver_listener: BackgroundLoop::spawn(event_receiver, move |event| {
                match event {
                    RangeInvalidated(r, Good) => {
                        valid_ranges_clone.write().unwrap().remove(&r);
                        good_ranges_clone.write().unwrap().add(r);
                    }
                    RangeInvalidated(r, Bad) => {
                        valid_ranges_clone.write().unwrap().remove(&r);
                        bad_ranges_clone.write().unwrap().add(r);
                    }
                    WorkerMessageSent(WorkerMessage {
                        point,
                        msg_type: Started,
                        ..
                    }) => active_clone
                        .write()
                        .unwrap()
                        .add(NumericRange::from_point(point)),
                    WorkerMessageSent(WorkerMessage {
                        point,
                        msg_type: Completed(_),
                        ..
                    }) => active_clone
                        .write()
                        .unwrap()
                        .remove(&NumericRange::from_point(point)),
                    _ => {}
                }
                DontCancel
            }),
        }
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        self.receiver_listener.cancel();
    }
}

fn range_color(
    good_ranges: &NumericRangeSet,
    bad_ranges: &NumericRangeSet,
    range: &NumericRange,
) -> Color {
    if good_ranges.contains_range(&range) {
        return Color::Green;
    } else if bad_ranges.contains_range(&range) {
        return Color::Red;
    }

    match (
        good_ranges.intersects_range(&range),
        bad_ranges.intersects_range(&range),
    ) {
        (false, false) => Color::Blue,
        _ => Color::Yellow,
    }
}

fn render_color_bar(
    good_ranges: &NumericRangeSet,
    bad_ranges: &NumericRangeSet,
    bounds: &NumericRange,
    active: &NumericRangeSet,
    width: usize,
) -> Line {
    let partitions = bounds.partition(width);

    let segments = partitions.into_iter().map(|r| {
        let color = range_color(good_ranges, bad_ranges, &r);
        let attributes = if active.intersects_range(&r) {
            Attributes::Blink | Attributes::Bold
        } else {
            Attributes::Bold
        };
        Segment::new("█".into(), color, attributes)
    });

    Line::new(segments)
}

fn render_bounds_bar(bounds: &NumericRange, width: usize) -> Vec<Line> {
    let (low, high) = unwrap_or!(bounds.as_tuple(), return Vec::new());
    let (low_s, high_s) = (low.to_string(), high.to_string());

    let l2 = unwrap_or!(
        Line::separate(low_s.into(), high_s.into(), width),
        return Vec::new()
    );

    let l1 = unwrap_or!(
        Line::separate("^".into(), "^".into(), width),
        return Vec::new()
    );

    return vec![l1, l2];
}

impl UiComponent for ProgressBar {
    fn render(&self, width: usize) -> Vec<Line> {
        let good_ranges = self.good_ranges.read().unwrap();
        let bad_ranges = self.bad_ranges.read().unwrap();
        let active = self.active.read().unwrap();

        let bounds = self.valid_ranges.read().unwrap().bounds();

        if bounds.is_empty() {
            return vec![Line::empty(), Line::empty()];
        }

        let mut ret = vec![render_color_bar(
            &good_ranges,
            &bad_ranges,
            &bounds,
            &active,
            width,
        )];

        ret.extend(render_bounds_bar(&bounds, width));

        ret
    }
}
