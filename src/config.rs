use std::collections::HashMap;

/// Top-level configuration for the diff algorithm.
pub struct DiffConfig {
    /// Controls how arrays are matched (by index or by distinguished key).
    pub match_config: MatchConfig,

    /// Controls behavior when multiple array elements match a distinguished key.
    pub ambiguous_strategy: AmbiguousMatchStrategy,
}

/// Configures how array elements are matched between actual and expected values.
///
/// By default, arrays are matched by index. Distinguished keys can be configured
/// per dot-separated path to enable name-based matching.
pub struct MatchConfig {
    /// Map from dot-separated path (e.g. `spec.containers`) to the distinguished
    /// key name (e.g. `name`).
    keys: HashMap<String, String>,
}

impl MatchConfig {
    /// Creates an empty config (index-based matching only).
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Adds a distinguished key for array elements at the given path.
    ///
    /// `path` is a dot-separated path like `spec.containers`.
    /// `key` is the field name to match on, like `name`.
    pub fn with_key_at(mut self, path: &str, key: &str) -> Self {
        self.keys.insert(path.to_owned(), key.to_owned());
        self
    }

    /// Returns the distinguished key for the given path, if configured.
    pub fn key_at(&self, path: &str) -> Option<&str> {
        self.keys.get(path).map(|s| s.as_str())
    }
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Controls behavior when multiple actual array elements match a single
/// expected element's distinguished key.
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

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            match_config: MatchConfig::default(),
            ambiguous_strategy: AmbiguousMatchStrategy::default(),
        }
    }
}
