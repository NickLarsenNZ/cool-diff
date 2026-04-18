use serde_json::Value;

/// The top-level result of a diff operation.
pub struct DiffTree {
    /// The root-level diff nodes.
    pub roots: Vec<DiffNode>,
}

impl DiffTree {
    /// Returns `true` if there are no differences.
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }
}

/// A node in the diff tree.
pub enum DiffNode {
    /// An intermediate node containing child diffs.
    Container {
        /// The path segment for this container.
        segment: PathSegment,

        /// Number of siblings/elements in the actual data not shown in the diff.
        omitted_count: u16,

        /// Child diff nodes.
        children: Vec<DiffNode>,
    },

    /// A terminal node representing a single difference.
    Leaf {
        /// The path segment for this leaf.
        segment: PathSegment,
        /// The kind of difference.
        kind: DiffKind,
    },
}

/// The kind of difference found at a leaf node.
pub enum DiffKind {
    /// Values differ but have the same type.
    Changed {
        /// The value that was actually present.
        actual: Value,

        /// The value that was expected.
        expected: Value,
    },

    /// A key or element is missing from the actual data.
    Missing {
        /// The expected value that was not found.
        expected: Value,
    },

    /// Values have different JSON types.
    TypeMismatch {
        /// The value that was actually present.
        actual: Value,

        /// Human-readable name of the actual type.
        actual_type: &'static str,

        /// The value that was expected.
        expected: Value,

        /// Human-readable name of the expected type.
        expected_type: &'static str,
    },
}

/// A segment in the path to a diff location.
pub enum PathSegment {
    /// An object key (e.g. `spec` in `spec.containers`).
    Key(String),

    /// An array element matched by a distinguished key (e.g. `name: FOO`).
    NamedElement {
        /// The key used to match (e.g. `name`).
        match_key: String,

        /// The value of the match key (e.g. `FOO`).
        match_value: String,
    },

    /// An array element matched by position.
    Index(u16),
}
