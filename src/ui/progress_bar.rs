use crate::messaging::listener::Listener;
use crate::parasect::event::Event;
use crate::parasect::event::Event::*;
use crate::parasect::types::ParasectPayloadAnswer::*;
use crate::parasect::worker::PointCompletionMessageType::*;
use crate::parasect::worker::WorkerMessage;
use crate::range::numeric_range::NumericRange;
use crate::range::numeric_range_set::NumericRangeSet;
use crate::ui::line::Line;
use crate::ui::segment::{Attributes, Color, Segment};
use crate::ui::ui_component::UiComponent;
use crate::util::macros::unwrap_or;
use crossbeam_channel::Receiver;
use ibig::UBig;
use std::sync::{Arc, RwLock};

/// Displays the progress of the parasect based on state inferred from the given event stream.
///
/// * With at least 4 `max_height`, displays a 2-high "color bar", colors representing the parasect state, with a 2-high "bounds bar" below it, displaying the minimum and maximum of the current search space. The cells of the color bar also blink when there is at least one worker currently active in that range.
/// * With 3 `max_height`, gets rid of the carats in the bounds bar.
/// * With 2 `max_height`, also reduces the color bar to 1 height.
/// * With 1 `max_height`, only displays the bounds bar numbers.
pub struct ProgressBar {
    _receiver_listener: Listener<'static, Event>,
    good_ranges: Arc<RwLock<NumericRangeSet>>,
    bad_ranges: Arc<RwLock<NumericRangeSet>>,
    valid_ranges: Arc<RwLock<NumericRangeSet>>,
    active: Arc<RwLock<NumericRangeSet>>,
}

impl ProgressBar {
    pub fn new(event_receiver: Receiver<Event>, initial_range: NumericRange) -> Self {
        let good_ranges = Arc::new(RwLock::new(NumericRangeSet::new()));
        let bad_ranges = Arc::new(RwLock::new(NumericRangeSet::new()));
        let active = Arc::new(RwLock::new(NumericRangeSet::new()));
        let valid_ranges = Arc::new(RwLock::new(NumericRangeSet::new()));

        valid_ranges.write().unwrap().add(initial_range);

        let good_ranges_clone = good_ranges.clone();
        let bad_ranges_clone = bad_ranges.clone();
        let active_clone = active.clone();
        let valid_ranges_clone = valid_ranges.clone();

        Self {
            good_ranges,
            bad_ranges,
            active,
            valid_ranges,
            _receiver_listener: Listener::spawn(event_receiver, move |event| match event {
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
            }),
        }
    }
}

/// Gets the color of a cell from the given range.
///
/// * Blue - Completely unknown.
/// * Yellow - Partially known, or a mix of good and bad.
/// * Green - All good.
/// * Red - All bad.
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

fn bound_partition_to_width(bounds: &NumericRange, width: usize) -> Vec<NumericRange> {
    if bounds.len() >= UBig::from(width) {
        bounds.partition(width)
    } else {
        let mut ret = Vec::new();

        let wparts = NumericRange::from_endpoints_excluding_end(0, width)
            .partition(usize::try_from(bounds.len()).unwrap());

        for (num, part) in bounds.iter().zip(wparts) {
            for _ in 0..usize::try_from(part.len()).unwrap() {
                ret.push(NumericRange::from_point(num.clone()));
            }
        }

        ret
    }
}

fn render_color_bar(
    good_ranges: &NumericRangeSet,
    bad_ranges: &NumericRangeSet,
    bounds: &NumericRange,
    active: &NumericRangeSet,
    width: usize,
) -> Line {
    let partitions = bound_partition_to_width(bounds, width);

    let segments = partitions.into_iter().map(|r| {
        let color = range_color(good_ranges, bad_ranges, &r);
        let attributes = if active.intersects_range(&r) {
            Attributes::Blink
        } else {
            Attributes::empty()
        };
        Segment::new("█".into(), color, attributes)
    });

    Line::new(segments)
}

fn render_bounds_bar(bounds: &NumericRange, width: usize, max_height: usize) -> Vec<Line> {
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

    match max_height {
        0 => Vec::new(),
        1 => vec![l2],
        _ => vec![l1, l2],
    }
}

impl UiComponent for ProgressBar {
    fn render(&self, width: usize, max_height: usize) -> Vec<Line> {
        let good_ranges = self.good_ranges.read().unwrap();
        let bad_ranges = self.bad_ranges.read().unwrap();
        let active = self.active.read().unwrap();

        let bounds = self.valid_ranges.read().unwrap().bounds();

        if bounds.is_empty() {
            return Vec::new();
        }

        let color_bar = render_color_bar(&good_ranges, &bad_ranges, &bounds, &active, width);

        let mut ret = Vec::new();

        for _ in 0..match max_height {
            0..=1 => 0,
            2 => 1,
            _ => 2,
        } {
            ret.push(color_bar.clone());
        }

        ret.extend(render_bounds_bar(
            &bounds,
            width,
            match max_height {
                0 => 0,
                1..=3 => 1,
                _ => 2,
            },
        ));

        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::collect_collection::CollectVec;
    use crate::parasect::types::ParasectPayloadResult::*;
    use crate::parasect::types::{ParasectPayloadAnswer, ParasectPayloadResult};
    use crate::test_util::test_util::test_util::{empty, ib, r};
    use crate::ui::line::mkline;
    use crossbeam_channel::unbounded;
    use ibig::IBig;
    use proptest::prelude::*;
    use proptest::proptest;
    use std::thread;
    use std::time::Duration;

    fn start(left: NumericRange, midpoint: IBig, right: NumericRange) -> Event {
        WorkerMessageSent(WorkerMessage {
            thread_id: 0,
            left: left,
            point: midpoint,
            right: right,
            msg_type: Started,
        })
    }

    fn stop(
        left: NumericRange,
        midpoint: IBig,
        right: NumericRange,
        result: ParasectPayloadResult,
    ) -> Event {
        WorkerMessageSent(WorkerMessage {
            thread_id: 0,
            left,
            point: midpoint,
            right,
            msg_type: Completed(result),
        })
    }

    fn r_invalid(range: NumericRange, answer: ParasectPayloadAnswer) -> Event {
        RangeInvalidated(range, answer)
    }

    fn progressbar_with_events<I: IntoIterator<Item = Event>>(
        initial_range: NumericRange,
        events: I,
    ) -> ProgressBar {
        let (send, recv) = unbounded();

        for e in events.into_iter() {
            send.send(e).unwrap();
        }

        let pb = ProgressBar::new(recv.clone(), initial_range);
        while !recv.is_empty() {
            thread::sleep(Duration::from_millis(2));
        }
        pb
    }

    fn assert_contents_eq<I: IntoIterator<Item = NumericRange>>(
        contents: &RwLock<NumericRangeSet>,
        expected: I,
    ) {
        let mut expected = expected.into_iter().collect_vec();
        expected.sort();

        assert_eq!(contents.read().unwrap().iter().collect_vec(), expected)
    }

    fn test_ranges<I: IntoIterator<Item = NumericRange>>(it: I) -> Arc<RwLock<NumericRangeSet>> {
        Arc::new(RwLock::new(NumericRangeSet::from_iter(it)))
    }

    #[test]
    fn test_progressbar_bar() {
        let (_send, recv) = unbounded();
        let mut pb = ProgressBar::new(recv, empty());

        pb.good_ranges = test_ranges([r(0, 4), r(10, 12), r(14, 17)]);
        pb.bad_ranges = test_ranges([r(19, 19), r(25, 34), r(40, 44)]);
        pb.active = test_ranges([r(6, 6), r(18, 18)]);
        pb.valid_ranges = test_ranges([
            r(-5, -1),
            r(5, 9),
            r(13, 13),
            r(18, 18),
            r(20, 24),
            r(35, 39),
        ]);

        let color_bar = mkline!(
            ("█", Color::Blue, Attributes::empty()),
            ("█", Color::Green, Attributes::empty()),
            ("█", Color::Blue, Attributes::Blink),
            ("█", Color::Yellow, Attributes::empty()),
            ("█", Color::Yellow, Attributes::Blink),
            ("█", Color::Blue, Attributes::empty()),
            ("██", Color::Red, Attributes::empty()),
            ("█", Color::Blue, Attributes::empty())
        );

        let expected = [
            color_bar.clone(),
            color_bar.clone(),
            "^       ^".into(),
            "-5     39".into(),
        ];
        let actual = pb.render(9, 4);

        assert_eq!(expected.into_iter().collect_vec(), actual);
    }

    #[test]
    fn test_progressbar_bar_3() {
        let (_send, recv) = unbounded();
        let mut pb = ProgressBar::new(recv, empty());

        pb.good_ranges = test_ranges([r(0, 4), r(10, 12), r(14, 17)]);
        pb.bad_ranges = test_ranges([r(19, 19), r(25, 34), r(40, 44)]);
        pb.active = test_ranges([r(6, 6), r(18, 18)]);
        pb.valid_ranges = test_ranges([
            r(-5, -1),
            r(5, 9),
            r(13, 13),
            r(18, 18),
            r(20, 24),
            r(35, 39),
        ]);

        let color_bar = mkline!(
            ("█", Color::Blue, Attributes::empty()),
            ("█", Color::Green, Attributes::empty()),
            ("█", Color::Blue, Attributes::Blink),
            ("█", Color::Yellow, Attributes::empty()),
            ("█", Color::Yellow, Attributes::Blink),
            ("█", Color::Blue, Attributes::empty()),
            ("██", Color::Red, Attributes::empty()),
            ("█", Color::Blue, Attributes::empty())
        );

        let expected = [color_bar.clone(), color_bar.clone(), "-5     39".into()];
        let actual = pb.render(9, 3);

        assert_eq!(expected.into_iter().collect_vec(), actual);
    }

    #[test]
    fn test_progressbar_bar_2() {
        let (_send, recv) = unbounded();
        let mut pb = ProgressBar::new(recv, empty());

        pb.good_ranges = test_ranges([r(0, 4), r(10, 12), r(14, 17)]);
        pb.bad_ranges = test_ranges([r(19, 19), r(25, 34), r(40, 44)]);
        pb.active = test_ranges([r(6, 6), r(18, 18)]);
        pb.valid_ranges = test_ranges([
            r(-5, -1),
            r(5, 9),
            r(13, 13),
            r(18, 18),
            r(20, 24),
            r(35, 39),
        ]);

        let color_bar = mkline!(
            ("█", Color::Blue, Attributes::empty()),
            ("█", Color::Green, Attributes::empty()),
            ("█", Color::Blue, Attributes::Blink),
            ("█", Color::Yellow, Attributes::empty()),
            ("█", Color::Yellow, Attributes::Blink),
            ("█", Color::Blue, Attributes::empty()),
            ("██", Color::Red, Attributes::empty()),
            ("█", Color::Blue, Attributes::empty())
        );

        let expected = [color_bar.clone(), "-5     39".into()];
        let actual = pb.render(9, 2);

        assert_eq!(expected.into_iter().collect_vec(), actual);
    }

    #[test]
    fn test_progressbar_bar_1() {
        let (_send, recv) = unbounded();
        let mut pb = ProgressBar::new(recv, empty());

        pb.good_ranges = test_ranges([r(0, 4), r(10, 12), r(14, 17)]);
        pb.bad_ranges = test_ranges([r(19, 19), r(25, 34), r(40, 44)]);
        pb.active = test_ranges([r(6, 6), r(18, 18)]);
        pb.valid_ranges = test_ranges([
            r(-5, -1),
            r(5, 9),
            r(13, 13),
            r(18, 18),
            r(20, 24),
            r(35, 39),
        ]);

        let expected = [Line::from("-5     39")];
        let actual = pb.render(9, 1);

        assert_eq!(expected.into_iter().collect_vec(), actual);
    }

    #[test]
    fn test_progressbar_bar_0() {
        let (_send, recv) = unbounded();
        let mut pb = ProgressBar::new(recv, empty());

        pb.good_ranges = test_ranges([r(0, 4), r(10, 12), r(14, 17)]);
        pb.bad_ranges = test_ranges([r(19, 19), r(25, 34), r(40, 44)]);
        pb.active = test_ranges([r(6, 6), r(18, 18)]);
        pb.valid_ranges = test_ranges([
            r(-5, -1),
            r(5, 9),
            r(13, 13),
            r(18, 18),
            r(20, 24),
            r(35, 39),
        ]);

        let actual = pb.render(9, 0);

        assert_eq!(Vec::<Line>::new(), actual);
    }

    #[test]
    fn test_progressbar_no_ranges_no_bar() {
        let (_send, recv) = unbounded();
        let mut pb = ProgressBar::new(recv, empty());

        pb.good_ranges = test_ranges([r(0, 4), r(10, 12), r(14, 17)]);
        pb.bad_ranges = test_ranges([r(19, 19), r(25, 34), r(40, 44)]);
        pb.active = test_ranges([r(6, 6), r(18, 18)]);
        pb.valid_ranges = test_ranges([]);

        assert_eq!(pb.render(9, 4), Vec::new());
    }

    #[test]
    fn test_progressbar_truncates_nums_when_too_small() {
        let (_send, recv) = unbounded();
        let mut pb = ProgressBar::new(recv, empty());

        pb.good_ranges = test_ranges([r(0, 4), r(10, 12), r(14, 17)]);
        pb.bad_ranges = test_ranges([r(19, 19), r(25, 34), r(40, 44)]);
        pb.active = test_ranges([r(6, 6), r(18, 18)]);
        pb.valid_ranges = test_ranges([
            r(-5, -1),
            r(5, 9),
            r(13, 13),
            r(18, 18),
            r(20, 24),
            r(35, 39),
        ]);

        let color_bar = Line::new([Segment::new("█".into(), Color::Yellow, Attributes::Blink)]);

        let expected = [color_bar.clone(), color_bar.clone()];
        let actual = pb.render(1, 4);

        assert_eq!(expected.into_iter().collect_vec(), actual);
    }

    #[test]
    fn test_progressbar_state() {
        let pb = progressbar_with_events(
            r(0, 39),
            [
                start(r(0, 19), ib(20), r(21, 39)),
                start(r(0, 9), ib(10), r(11, 19)),
                start(r(21, 29), ib(30), r(31, 39)),
                stop(r(0, 9), ib(10), r(11, 19), Continue(Good)),
                r_invalid(r(0, 10), Good),
                stop(r(0, 19), ib(20), r(21, 39), Continue(Bad)),
                r_invalid(r(20, 39), Bad),
                start(r(11, 14), ib(15), r(16, 19)),
                stop(r(11, 14), ib(15), r(16, 19), Continue(Good)),
                r_invalid(r(11, 15), Good),
                start(r(16, 17), ib(18), r(19, 19)),
            ],
        );

        assert_contents_eq(&pb.active, [r(18, 18), r(30, 30)]);
        assert_contents_eq(&pb.bad_ranges, [r(20, 39)]);
        assert_contents_eq(&pb.good_ranges, [r(0, 15)]);
        assert_contents_eq(&pb.valid_ranges, [r(16, 19)]);
    }

    #[test]
    fn test_bound_partition_snug() {
        let bounds = r(0, 4);

        assert_eq!(
            bound_partition_to_width(&bounds, 5),
            vec![r(0, 0), r(1, 1), r(2, 2), r(3, 3), r(4, 4)]
        )
    }

    #[test]
    fn test_bound_partition_thicc() {
        let bounds = r(0, 16);

        assert_eq!(
            bound_partition_to_width(&bounds, 5),
            vec![r(0, 3), r(4, 7), r(8, 10), r(11, 13), r(14, 16)]
        )
    }

    #[test]
    fn test_bound_partition_thin() {
        let bounds = r(0, 2);

        assert_eq!(
            bound_partition_to_width(&bounds, 5),
            vec![r(0, 0), r(0, 0), r(1, 1), r(1, 1), r(2, 2)]
        )
    }

    proptest! {
        #[test]
        fn test_bound_partition_equals_width(a in 1..2000, b in 1..2000, c in 1..1000usize) {
            prop_assume!(a < b);

            assert_eq!(bound_partition_to_width(&r(a, b), c).len(), c);
        }
    }
}
