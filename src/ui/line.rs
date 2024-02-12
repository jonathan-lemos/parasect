use crate::collections::collect_collection::CollectVec;

#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Color {
    #[default]
    Default,
    Red,
    Green,
    Yellow,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Line {
    segments: Box<[(String, Color)]>,
}

impl Line {
    pub fn new(segments: Box<[(String, Color)]>) -> Self {
        Self { segments }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, Color)> {
        self.segments.iter()
    }

    pub fn len(&self) -> usize {
        self.segments.iter().fold(0, |a, c| a + c.0.len())
    }

    pub fn truncate(&self, width: usize) -> Self {
        let mut new_segments = Vec::new();
        let mut total_len = 0usize;

        for (content, color) in self.segments.iter() {
            if total_len + content.len() == width {
                new_segments.push((content.clone(), *color));
                break;
            }

            if total_len + content.len() > width {
                let remaining = width - total_len;
                let mut new_content = (&content[..remaining - 1]).to_string();
                new_content.push('…');
                new_segments.push((new_content, *color));
                break;
            }

            new_segments.push((content.clone(), *color));
            total_len += content.len();
        }

        Self::new(new_segments.into_boxed_slice())
    }
}

impl From<String> for Line {
    fn from(value: String) -> Self {
        Self::from((value, Color::Default))
    }
}

impl From<(String, Color)> for Line {
    fn from(value: (String, Color)) -> Self {
        Self::new(Box::new([value]))
    }
}

impl FromIterator<(String, Color)> for Line {
    fn from_iter<T: IntoIterator<Item = (String, Color)>>(iter: T) -> Self {
        Self::new(iter.into_iter().collect_vec().into_boxed_slice())
    }
}

impl FromIterator<String> for Line {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self::new(
            iter.into_iter()
                .map(|x| (x, Color::Default))
                .collect_vec()
                .into_boxed_slice(),
        )
    }
}
