pub mod yaml;

use crate::model::DiffTree;

/// Renders a `DiffTree` into a human-readable string.
pub trait DiffRenderer {
    fn render(&self, tree: &DiffTree) -> String;
}

/// Line prefix characters for diff output.
pub mod indicator {
    /// Unchanged context lines.
    pub const UNCHANGED: char = ' ';

    /// Expected values (what we wanted but didn't get).
    pub const EXPECTED: char = '-';

    /// Actual values (what we got instead).
    pub const ACTUAL: char = '+';
}
