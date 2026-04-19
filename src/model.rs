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

impl DiffNode {
    /// Creates a leaf node representing a single difference.
    pub fn leaf(segment: PathSegment, kind: DiffKind) -> Self {
        Self::Leaf { segment, kind }
    }

    /// Creates a container node with child diffs.
    pub fn container(segment: PathSegment, omitted_count: u16, children: Vec<DiffNode>) -> Self {
        Self::Container {
            segment,
            omitted_count,
            children,
        }
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

impl DiffKind {
    /// Creates a `Changed` diff for values of the same type that differ.
    pub fn changed(actual: Value, expected: Value) -> Self {
        Self::Changed { actual, expected }
    }

    /// Creates a `Missing` diff for a value not found in actual.
    pub fn missing(expected: Value) -> Self {
        Self::Missing { expected }
    }

    /// Creates a `TypeMismatch` diff for values with different JSON types.
    pub fn type_mismatch(
        actual: Value,
        actual_type: &'static str,
        expected: Value,
        expected_type: &'static str,
    ) -> Self {
        Self::TypeMismatch {
            actual,
            actual_type,
            expected,
            expected_type,
        }
    }
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

impl PathSegment {
    /// Returns true if this segment represents an array element.
    pub fn is_array(&self) -> bool {
        matches!(
            self,
            PathSegment::NamedElement { .. } | PathSegment::Index(_) | PathSegment::Unmatched
        )
    }
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

    /// An expected array element with no matching actual element.
    ///
    /// Used with key-based and contains matching when no candidate was
    /// found in the actual array.
    Unmatched,
}
