use crate::model::DiffTree;

/// Renders a `DiffTree` into a human-readable string.
pub trait DiffRenderer {
    fn render(&self, tree: &DiffTree) -> String;
}
