use std::collections::HashMap;

/// Top-level configuration for the diff algorithm.
pub struct DiffConfig {
    /// Controls how arrays are matched at each path.
    pub match_config: MatchConfig,

    /// Controls behavior when multiple array elements could match.
    pub ambiguous_strategy: AmbiguousMatchStrategy,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            match_config: MatchConfig::default(),
            ambiguous_strategy: AmbiguousMatchStrategy::default(),
        }
    }
}

/// Configures how array elements are matched between actual and expected values.
///
/// By default, arrays are matched by index. Each path can be configured with
/// a different matching mode via [`ArrayMatchMode`].
pub struct MatchConfig {
    /// Map from dot-separated path (e.g. `spec.containers`) to the array
    /// matching mode for that path.
    modes: HashMap<String, ArrayMatchMode>,
}

impl MatchConfig {
    /// Creates an empty config (index-based matching for all arrays).
    pub fn new() -> Self {
        Self {
            modes: HashMap::new(),
        }
    }

    /// Sets the array matching mode for the given dot-separated path.
    pub fn with_mode_at(mut self, path: &str, mode: ArrayMatchMode) -> Self {
        self.modes.insert(path.to_owned(), mode);
        self
    }

    /// Returns the array matching mode for the given path, defaulting to
    /// [`ArrayMatchMode::Index`] if not configured.
    pub fn mode_at(&self, path: &str) -> &ArrayMatchMode {
        self.modes
            .get(path)
            .unwrap_or(&ArrayMatchMode::Index)
    }
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// How array elements are matched between actual and expected values at a
/// given path.
pub enum ArrayMatchMode {
    /// Match by position (default). Element 0 compares to element 0, etc.
    Index,

    /// Match by a distinguished key field (e.g. `name`). Scans the actual
    /// array for an element with a matching key value.
    Key(String),

    /// Find a matching element anywhere in the actual array. Uses exact
    /// value comparison for scalars, recursive subset matching for objects.
    Contains,
}

/// Controls behavior when multiple actual array elements could match a single
/// expected element.
pub enum AmbiguousMatchStrategy {
    /// Fail if more than one candidate exists.
    Strict,

    /// Pick the candidate with the fewest diffs, with a warning comment.
    BestMatch,

    /// Pick the candidate with the fewest diffs, without a warning comment.
    Silent,
}

impl Default for AmbiguousMatchStrategy {
    fn default() -> Self {
        Self::Strict
    }
}
