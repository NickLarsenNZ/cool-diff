use std::collections::HashMap;

/// Top-level configuration for the diff algorithm.
pub struct DiffConfig {
    /// Controls how arrays are matched at each path.
    match_config: MatchConfig,

    /// Default array match mode, used when a path does not specify its own.
    default_array_mode: ArrayMatchMode,

    /// Default ambiguity strategy, used when a path does not specify its own.
    default_ambiguous_strategy: AmbiguousMatchStrategy,
}

impl DiffConfig {
    /// Creates a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the match config for array path lookups.
    pub fn with_match_config(mut self, match_config: MatchConfig) -> Self {
        self.match_config = match_config;
        self
    }

    /// Sets the fallback array match mode for paths without explicit config.
    pub fn with_fallback_array_mode(mut self, mode: ArrayMatchMode) -> Self {
        self.default_array_mode = mode;
        self
    }

    /// Sets the fallback ambiguity strategy for paths without explicit config.
    pub fn with_fallback_ambiguous_strategy(mut self, strategy: AmbiguousMatchStrategy) -> Self {
        self.default_ambiguous_strategy = strategy;
        self
    }

    /// Returns the match config.
    pub fn match_config(&self) -> &MatchConfig {
        &self.match_config
    }

    /// Returns the default array match mode.
    pub fn default_array_mode(&self) -> &ArrayMatchMode {
        &self.default_array_mode
    }

    /// Returns the default ambiguity strategy.
    pub fn default_ambiguous_strategy(&self) -> &AmbiguousMatchStrategy {
        &self.default_ambiguous_strategy
    }
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            match_config: MatchConfig::default(),
            default_array_mode: ArrayMatchMode::Index,
            default_ambiguous_strategy: AmbiguousMatchStrategy::default(),
        }
    }
}

/// Configures how array elements are matched between actual and expected values.
///
/// By default, arrays are matched by index. Each path can be configured with
/// a different matching mode and ambiguity strategy via [`ArrayMatchConfig`].
pub struct MatchConfig {
    /// Map from dot-separated path (e.g. `spec.containers`) to the array
    /// match configuration for that path.
    paths: HashMap<String, ArrayMatchConfig>,
}

impl MatchConfig {
    /// Creates an empty config (index-based matching for all arrays).
    pub fn new() -> Self {
        Self {
            paths: HashMap::new(),
        }
    }

    /// Sets the array match configuration for the given dot-separated path.
    pub fn with_config_at(mut self, path: &str, config: ArrayMatchConfig) -> Self {
        self.paths.insert(path.to_owned(), config);
        self
    }

    /// Returns the array match configuration for the given path, if configured.
    pub fn config_at(&self, path: &str) -> Option<&ArrayMatchConfig> {
        self.paths.get(path)
    }
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-path configuration for array element matching.
pub struct ArrayMatchConfig {
    /// How elements are matched at this path.
    mode: ArrayMatchMode,

    /// Optional override for the ambiguity strategy at this path.
    ///
    /// Falls back to [`DiffConfig::default_ambiguous_strategy`] if `None`.
    ambiguous_strategy: Option<AmbiguousMatchStrategy>,
}

impl ArrayMatchConfig {
    /// Creates a config with the given mode and no ambiguity override.
    pub fn new(mode: ArrayMatchMode) -> Self {
        Self {
            mode,
            ambiguous_strategy: None,
        }
    }

    /// Sets the ambiguity strategy override for this path.
    pub fn with_ambiguous_strategy(mut self, strategy: AmbiguousMatchStrategy) -> Self {
        self.ambiguous_strategy = Some(strategy);
        self
    }

    /// Returns the array match mode.
    pub fn mode(&self) -> &ArrayMatchMode {
        &self.mode
    }

    /// Returns the ambiguity strategy override, if set.
    pub fn ambiguous_strategy(&self) -> Option<&AmbiguousMatchStrategy> {
        self.ambiguous_strategy.as_ref()
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
#[derive(Default)]
pub enum AmbiguousMatchStrategy {
    /// Fail if more than one candidate exists.
    #[default]
    Strict,

    /// Pick the candidate with the fewest diffs, with a warning comment.
    BestMatch,

    /// Pick the candidate with the fewest diffs, without a warning comment.
    Silent,
}
