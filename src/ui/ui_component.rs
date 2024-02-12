use crate::ui::line::Line;

pub trait UiComponent {
    fn render(&self, width: usize) -> Vec<Line>;
}
