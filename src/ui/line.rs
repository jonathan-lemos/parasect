use crate::collections::collect_collection::CollectVec;
use crate::ui::segment::Segment;

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
        self.plaintext().len()
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
                new_segments.push(seg.map_content(|c| {
                    let mut s = c[..remaining - 1].to_string();
                    s.push('…');
                    s
                }));
                break;
            }

            total_len += seg.len();
            new_segments.push(seg.clone());
        }

        Self::new(new_segments)
    }
}

impl<I: Into<Segment>> From<I> for Line {
    fn from(value: I) -> Self {
        Line::new([value.into()])
    }
}

impl<I: Into<Segment>> FromIterator<I> for Line {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Line::new(iter.into_iter().map(|x| x.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let line = Line::new([
            Segment::new("amog".into(), Color::Blue, Attributes::Bold),
            Segment::new("us".into(), Color::Blue, Attributes::Bold),
            Segment::new("sus".into(), Color::Green, Attributes::Blink),
            Segment::new("amog".into(), Color::Red, Attributes::Bold),
            Segment::new("us".into(), Color::Red, Attributes::Bold),
        ]);

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
        let line1 = Line::from_iter([
            Segment::new("amog".into(), Color::Red, Attributes::Blink),
            Segment::new("us".into(), Color::Green, Attributes::Bold),
        ]);

        let line2 = Line::from(Segment::new(
            "sus".into(),
            Color::Green,
            Attributes::empty(),
        ));

        assert_eq!(
            Line::separate(line1, line2, 12),
            Some(Line::from_iter([
                Segment::new("amog".into(), Color::Red, Attributes::Blink),
                Segment::new("us".into(), Color::Green, Attributes::Bold),
                "   ".into(),
                Segment::new("sus".into(), Color::Green, Attributes::empty())
            ]))
        )
    }

    #[test]
    pub fn test_separate_too_close() {
        let line1 = Line::from_iter([
            Segment::new("amog".into(), Color::Red, Attributes::Blink),
            Segment::new("us".into(), Color::Green, Attributes::Bold),
        ]);

        let line2 = Line::from(Segment::new(
            "sus".into(),
            Color::Green,
            Attributes::empty(),
        ));

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
        let line = Line::from_iter([
            Segment::new("amog".into(), Color::Red, Attributes::Blink),
            Segment::new("us".into(), Color::Green, Attributes::Bold),
        ]);

        assert_eq!(
            line.center(9),
            Line::from_iter([
                " ".into(),
                Segment::new("amog".into(), Color::Red, Attributes::Blink),
                Segment::new("us".into(), Color::Green, Attributes::Bold),
            ])
        );
    }

    #[test]
    pub fn test_center_formatted_eq_len() {
        let line = Line::from_iter([
            Segment::new("amog".into(), Color::Red, Attributes::Blink),
            Segment::new("us".into(), Color::Green, Attributes::Bold),
        ]);

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
        let line = Line::from_iter([
            Segment::new("amog".into(), Color::Red, Attributes::Blink),
            Segment::new("us".into(), Color::Green, Attributes::Bold),
        ]);

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
            Line::from_iter([
                Segment::new("amog".into(), Color::Red, Attributes::Blink),
                Segment::new("us".into(), Color::Green, Attributes::Bold),
                "   ".into(),
            ])
        );
    }

    #[test]
    pub fn test_pad_eq_formatted() {
        let line = Line::from_iter([
            Segment::new("amog".into(), Color::Red, Attributes::Blink),
            Segment::new("us".into(), Color::Green, Attributes::Bold),
        ]);

        assert_eq!(line.pad(6), line);
    }

    #[test]
    pub fn test_plaintext() {
        let line = Line::from_iter([
            Segment::new("amog".into(), Color::Red, Attributes::Blink),
            Segment::new("us".into(), Color::Green, Attributes::Bold),
        ]);

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
        let line = Line::from_iter([
            Segment::new("sus".into(), Color::Red, Attributes::Blink),
            Segment::new("amogus".into(), Color::Green, Attributes::Bold),
        ]);

        assert_eq!(
            line.truncate(6),
            Line::from_iter([
                Segment::new("sus".into(), Color::Red, Attributes::Blink),
                Segment::new("am…".into(), Color::Green, Attributes::Bold),
            ])
        );

        assert_eq!(
            line.truncate(4),
            Line::from_iter([
                Segment::new("sus".into(), Color::Red, Attributes::Blink),
                Segment::new("…".into(), Color::Green, Attributes::Bold),
            ])
        );

        assert_eq!(
            line.truncate(3),
            Line::from_iter([Segment::new("su…".into(), Color::Red, Attributes::Blink),])
        );
    }
}
