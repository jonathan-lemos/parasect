use crate::collections::collect_collection::CollectVec;
use crate::ui::segment::Segment;
use crate::ui::ui_component::UiComponent;
use unicode_segmentation::UnicodeSegmentation;

/// A line of (optionally styled) text for printing to a TTY.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Line {
    segments: Box<[Segment]>,
}

impl Line {
    pub fn new<I: IntoIterator<Item = Segment>>(segments: I) -> Self {
        let mut segs = Vec::new();

        for seg in segments.into_iter() {
            if seg.content().is_empty() {
                continue;
            }

            if seg.content().contains('\n') {
                panic!("Lines cannot contain newlines in them.");
            }

            match segs.last() {
                None => segs.push(seg),
                Some(s) => {
                    if s.color() == seg.color() && s.attributes() == seg.attributes() {
                        let old = segs.pop().unwrap();
                        segs.push(seg.map_content(|x| old.content().to_string() + x));
                    } else {
                        segs.push(seg)
                    }
                }
            }
        }

        Self {
            segments: segs.into_boxed_slice(),
        }
    }

    /// Returns a line that clears the given width.
    pub fn blank(width: usize) -> Self {
        Self::from(" ".repeat(width))
    }

    /// Returns an empty line. It has no effect when printed.
    pub fn empty() -> Self {
        Self::new([])
    }

    /// Joins the given lines into one line.
    pub fn join<I: IntoIterator<Item = Line>>(lines: I) -> Self {
        let mut joined = Vec::<Segment>::new();

        for line in lines.into_iter() {
            joined.extend(line.iter().map(|x| x.clone()).collect_vec());
        }

        Self::new(joined)
    }

    /// Returns the two lines separated by as many spaces as necessary to fill the `width`.
    ///
    /// Returns None if no spaces can be inserted while remaining in the given `width`.
    pub fn separate(l1: Line, l2: Line, width: usize) -> Option<Self> {
        if l1.len() + l2.len() >= width {
            return None;
        }

        let spaces_between = width - (l1.len() + l2.len());

        return Some(Self::join([l1, " ".repeat(spaces_between).into(), l2]));
    }

    /// Centers the line's contents within the given `width`.
    ///
    /// Returns a clone of the input if its length >= `width`.
    /// Note: this will only pad spaces at the beginning of the string.
    pub fn center(&self, width: usize) -> Self {
        if self.len() >= width {
            return self.clone();
        }

        let spaces = width - self.len();
        let spaces_left = spaces / 2;

        Self::join([" ".repeat(spaces_left).into(), self.clone()])
    }

    /// Iterates through the segments in the line.
    pub fn iter(&self) -> impl Iterator<Item = &Segment> {
        self.segments.iter()
    }

    /// Returns the number of characters in the line.
    pub fn len(&self) -> usize {
        self.plaintext().graphemes(true).count()
    }

    /// Pads the end of th eline with spaces to the given `width`.
    ///
    /// The original line is returned if it exceeds width in length.
    pub fn pad(&self, width: usize) -> Self {
        if self.len() >= width {
            return self.clone();
        }

        let spaces = width - self.len();
        Self::join([self.clone(), " ".repeat(spaces).into()])
    }

    /// Returns the raw text of the Line with no coloring or attributes.
    pub fn plaintext(&self) -> String {
        self.iter().map(|x| x.content()).collect()
    }

    /// Prints the line to stdout, without a newline at the end.
    pub fn print(&self) {
        for seg in self.segments.iter() {
            seg.print();
        }
    }

    /// Truncates the line's contents to the given width.
    pub fn truncate(&self, width: usize) -> Self {
        if self.len() <= width {
            return self.clone();
        }

        let mut new_segments = Vec::new();
        let mut total_len = 0usize;

        for seg in self.segments.iter() {
            if total_len + seg.len() >= width {
                let remaining = width - total_len;
                new_segments.push(seg.map_content(|c| c[..remaining - 1].to_string() + "…"));
                break;
            }

            total_len += seg.len();
            new_segments.push(seg.clone());
        }

        Self::new(new_segments)
    }
}

fn print_lines_notty<'a, I: IntoIterator<Item = &'a Line>>(lines: I) {
    for line in lines {
        println!("{}", line.plaintext());
    }
}

pub fn print_lines<'a, I: IntoIterator<Item = &'a Line>>(lines: I) {
    let width = match termion::terminal_size() {
        Err(_) => return print_lines_notty(lines),
        Ok((_, w)) => w as usize,
    };

    for line in lines {
        let line = line.pad(width);
        line.print();

        if line.len() != width {
            println!();
        }
    }
}

impl<I: Into<Segment>> From<I> for Line {
    fn from(value: I) -> Self {
        Self::from_iter([value.into()])
    }
}

impl<I: Into<Segment>> FromIterator<I> for Line {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Line::new(iter.into_iter().map(|x| x.into()))
    }
}

impl UiComponent for Line {
    fn render(&self, width: usize, max_height: usize) -> Vec<Line> {
        if max_height > 0 {
            vec![self.truncate(width)]
        } else {
            Vec::new()
        }
    }
}

/// Makes a Line from zero or more comma-separated `Into<Segment>`.
macro_rules! mkline {
    ($($arg:expr),*) => {
        crate::ui::line::Line::from_iter([$(crate::ui::segment::Segment::from($arg),)*])
    };
}

pub(crate) use mkline;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::segment::Color::Default;
    use crate::ui::segment::{Attributes, Color};

    #[test]
    pub fn test_blank() {
        let blank = Line::blank(5);
        assert_eq!(blank, "     ".into());
    }

    #[test]
    pub fn test_empty() {
        let blank = Line::empty();
        assert_eq!(blank, "".into());
    }

    #[test]
    pub fn test_new_condenses() {
        let line = mkline!(
            ("amog", Color::Blue, Attributes::Bold),
            ("us", Color::Blue, Attributes::Bold),
            ("sus", Color::Green, Attributes::Blink),
            ("amog", Color::Red, Attributes::Bold),
            ("us", Color::Red, Attributes::Bold)
        );

        assert_eq!(
            line.segments,
            [
                Segment::new("amogus".into(), Color::Blue, Attributes::Bold),
                Segment::new("sus".into(), Color::Green, Attributes::Blink),
                Segment::new("amogus".into(), Color::Red, Attributes::Bold),
            ]
            .as_slice()
            .into()
        )
    }

    #[test]
    pub fn test_separate() {
        let line1 = mkline!(
            ("amog", Color::Red, Attributes::Blink),
            ("us", Color::Green, Attributes::Bold)
        );

        let line2 = Line::from(("sus", Color::Green, Attributes::empty()));

        assert_eq!(
            Line::separate(line1, line2, 12),
            Some(mkline!(
                ("amog", Color::Red, Attributes::Blink),
                ("us", Color::Green, Attributes::Bold),
                "   ",
                ("sus", Color::Green, Attributes::empty())
            ))
        )
    }

    #[test]
    pub fn test_separate_too_close() {
        let line1 = mkline!(
            ("amog", Color::Red, Attributes::Blink),
            ("us", Color::Green, Attributes::Bold)
        );

        let line2 = Line::from(("sus", Color::Green, Attributes::empty()));

        assert_eq!(Line::separate(line1, line2, 9), None)
    }

    #[test]
    pub fn test_center_odd() {
        let line = Line::from("foo");
        assert_eq!(line.center(8), "  foo".into());
    }

    #[test]
    pub fn test_center_even() {
        let line = Line::from("foo");
        assert_eq!(line.center(9), "   foo".into());
    }

    #[test]
    pub fn test_center_formatted() {
        let line = mkline!(
            ("amog", Color::Red, Attributes::Blink),
            ("us", Color::Green, Attributes::Bold)
        );

        assert_eq!(
            line.center(9),
            mkline!(
                " ",
                ("amog", Color::Red, Attributes::Blink),
                ("us", Color::Green, Attributes::Bold)
            )
        );
    }

    #[test]
    pub fn test_center_formatted_eq_len() {
        let line = mkline!(
            ("amog", Color::Red, Attributes::Blink),
            ("us", Color::Green, Attributes::Bold)
        );

        assert_eq!(line.center(6), line);
    }

    #[test]
    pub fn test_center_too_big() {
        let line = Line::from("foobar");
        assert_eq!(line.center(5), "foobar".into());
    }

    #[test]
    pub fn test_len() {
        let line = Line::from_iter(["foo", "bar", "69"]);
        assert_eq!(line.len(), 8);
    }

    #[test]
    pub fn test_len_formatted() {
        let line = mkline!(
            ("amog", Color::Red, Attributes::Blink),
            ("us", Color::Green, Attributes::Bold)
        );

        assert_eq!(line.len(), 6);
    }

    #[test]
    pub fn test_pad() {
        let line = Line::from("foo");
        assert_eq!(line.pad(8), "foo     ".into());
    }

    #[test]
    pub fn test_pad_le_width() {
        let line = Line::from("foo");
        assert_eq!(line.pad(3), "foo".into());
        assert_eq!(line.pad(2), "foo".into());
    }

    #[test]
    pub fn test_pad_formatted() {
        let line = Line::from_iter([
            Segment::new("amog".into(), Color::Red, Attributes::Blink),
            Segment::new("us".into(), Color::Green, Attributes::Bold),
        ]);

        assert_eq!(
            line.pad(9),
            mkline!(
                ("amog", Color::Red, Attributes::Blink),
                ("us", Color::Green, Attributes::Bold),
                "   "
            )
        );
    }

    #[test]
    pub fn test_pad_eq_formatted() {
        let line = mkline!(
            ("amog", Color::Red, Attributes::Blink),
            ("us", Color::Green, Attributes::Bold)
        );

        assert_eq!(line.pad(6), line);
    }

    #[test]
    pub fn test_plaintext() {
        let line = mkline!(
            ("amog", Color::Red, Attributes::Blink),
            ("us", Color::Green, Attributes::Bold)
        );

        assert_eq!(line.plaintext(), "amogus");
    }

    #[test]
    pub fn test_truncate() {
        let line = Line::from("123456789");
        assert_eq!(line.truncate(3), "12…".into());
    }

    #[test]
    pub fn test_truncate_ge_width() {
        let line = Line::from("123456789");
        assert_eq!(line.truncate(9), "123456789".into());
        assert_eq!(line.truncate(10), "123456789".into());
    }

    #[test]
    pub fn test_truncate_formatted() {
        let line = mkline!(
            ("sus", Color::Red, Attributes::Blink),
            ("amogus", Color::Green, Attributes::Bold)
        );

        assert_eq!(
            line.truncate(9),
            mkline!(
                ("sus", Color::Red, Attributes::Blink),
                ("amogus", Color::Green, Attributes::Bold)
            )
        );

        assert_eq!(
            line.truncate(6),
            mkline!(
                ("sus", Color::Red, Attributes::Blink),
                ("am…", Color::Green, Attributes::Bold)
            )
        );

        assert_eq!(
            line.truncate(4),
            mkline!(
                ("sus", Color::Red, Attributes::Blink),
                ("…", Color::Green, Attributes::Bold)
            )
        );

        assert_eq!(
            line.truncate(3),
            mkline!(("su…", Color::Red, Attributes::Blink))
        );
    }

    #[test]
    fn test_mkline() {
        let m = mkline!(
            ("sus", Color::Red),
            " ",
            ("amogus", Color::Green, Attributes::Bold)
        );
        let segs = [
            Segment::new("sus".into(), Color::Red, Attributes::empty()),
            Segment::new(" ".into(), Default, Attributes::empty()),
            Segment::new("amogus".into(), Color::Green, Attributes::Bold),
        ]
        .into_iter()
        .collect_vec()
        .into_boxed_slice();

        assert_eq!(m.segments, segs)
    }

    #[test]
    fn test_debug_line_render() {
        let line = mkline!(
            ("sus", Color::Red),
            ("amogus", Color::Yellow, Attributes::Bold)
        );

        print_lines([&line]);
    }
}
