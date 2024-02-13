use crate::collections::collect_collection::CollectVec;
use crate::ui::segment::Segment;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Line {
    segments: Box<[Segment]>,
}

impl Line {
    pub fn empty() -> Self {
        Self {
            segments: Box::new([]),
        }
    }

    pub fn new<I: IntoIterator<Item = Segment>>(segments: I) -> Self {
        Self {
            segments: segments.into_iter().collect_vec().into_boxed_slice(),
        }
    }

    pub fn join<I: IntoIterator<Item = Line>>(lines: I) -> Self {
        let mut joined = Vec::<Segment>::new();

        for line in lines.into_iter() {
            joined.append(&mut line.iter().collect_vec());
        }

        Self {
            segments: joined.into_boxed_slice(),
        }
    }

    pub fn separate(l1: Line, l2: Line, width: usize) -> Option<Self> {
        if l1.len() + l2.len() >= width {
            return None;
        }

        let spaces_between = width - (l1.len() + l2.len());

        return Some(Self::join([l1, " ".repeat(spaces_between).into(), l2]));
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = Segment> + 'a {
        self.segments.iter().map(|x| x.clone())
    }

    pub fn len(&self) -> usize {
        self.segments.iter().fold(0, |a, c| a + c.0.len())
    }

    pub fn center(&self, width: usize) -> Self {
        if self.len() >= width {
            return self.clone();
        }

        let spaces = width - self.len();
        let spaces_left = spaces / 2;

        Self::join([" ".repeat(spaces_left).into(), self.clone()])
    }

    pub fn truncate(&self, width: usize) -> Self {
        let mut new_segments = Vec::new();
        let mut total_len = 0usize;

        for seg in self.segments.iter() {
            if total_len + seg.len() == width {
                new_segments.push(seg.clone());
                break;
            }

            if total_len + seg.len() > width {
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

impl FromIterator<Segment> for Line {
    fn from_iter<T: IntoIterator<Item = Segment>>(iter: T) -> Self {
        Line::new(iter)
    }
}
