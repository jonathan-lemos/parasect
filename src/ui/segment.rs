use crate::ui::segment::Color::*;
use bitflags::bitflags;
use termion::{color, style};

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy, Default)]
pub enum Color {
    #[default]
    Default,
    Red,
    Green,
    Yellow,
    Blue,
}

impl Color {
    pub fn render(&self) {
        match self {
            Default => print!("{}", color::Fg(color::Reset)),
            Red => print!("{}", color::Fg(color::LightRed)),
            Yellow => print!("{}", color::Fg(color::LightYellow)),
            Green => print!("{}", color::Fg(color::LightGreen)),
            Blue => print!("{}", color::Fg(color::LightBlue)),
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
    pub struct Attributes: u32 {
        const Blink = 0x1;
        const Bold = 0x2;
    }
}

impl Attributes {
    pub fn render(&self) {
        for attr in self.iter() {
            match attr {
                Attributes::Blink => print!("{}", style::Blink),
                Attributes::Bold => print!("{}", style::Bold),
                _ => {}
            }
        }
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct Segment {
    content: String,
    color: Color,
    attributes: Attributes,
}

impl Segment {
    pub fn new(content: String, color: Color, attributes: Attributes) -> Self {
        Self {
            content,
            color,
            attributes,
        }
    }

    pub fn map_content<F: FnOnce(&str) -> String>(&self, mapper: F) -> Self {
        Self::new(mapper(&self.content), self.color, self.attributes)
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn render(&self) {
        for attr in self.attributes.iter() {
            attr.render();
        }
        print!(
            "{}{}{}",
            self.content,
            color::Fg(color::Reset),
            style::Reset,
        );
    }
}

impl From<&str> for Segment {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<String> for Segment {
    fn from(value: String) -> Self {
        Self::new(value, Default, Attributes::empty())
    }
}
