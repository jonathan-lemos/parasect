use crate::collections::collect_collection::CollectVec;
use crate::ui::line::Line;

pub trait UiComponent {
    /// Turns the UiComponent into lines of no more than the given `width` and given `max_height`.
    fn render(&self, width: usize, max_height: usize) -> Vec<Line>;
}

impl<C: UiComponent> UiComponent for &[C] {
    fn render(&self, width: usize, max_height: usize) -> Vec<Line> {
        self.iter()
            .flat_map(|x| x.render(width, max_height))
            .collect_vec()
    }
}
