//! Compact, context-preserving diffs of structured data.
//!
//! `cool-diff` compares two [`serde_json::Value`] trees and produces a minimal,
//! human-readable diff. It is format-agnostic at the core and ships with a
//! YAML-style renderer ([`YamlRenderer`]) out of the box.
//!
//! # Quick start
//!
//! ```
//! use cool_diff::{diff, DiffConfig, DiffRenderer as _, YamlRenderer};
//!
//! let actual = serde_json::json!({
//!     "server": {
//!         "host": "0.0.0.0",
//!         "port": 8080,
//!         "tls": true,
//!     }
//! });
//! let expected = serde_json::json!({
//!     "server": {
//!         "port": 3000,
//!     }
//! });
//!
//! let tree = diff(&actual, &expected, &DiffConfig::default()).unwrap();
//!
//! if !tree.is_empty() {
//!     let output = YamlRenderer::new().render(&tree);
//!     print!("{output}");
//! }
//! ```
//!
//! # Array matching
//!
//! By default, arrays are compared by position (index). You can configure
//! per-path matching via [`MatchConfig`]:
//!
//! - [`ArrayMatchMode::Index`] - match by position (default)
//! - [`ArrayMatchMode::Key`] - match by a configured distinguished field
//! - [`ArrayMatchMode::Contains`] - find a matching element anywhere
//!
//! See [`DiffConfig`] and [`ArrayMatchConfig`] for configuration options.

#![deny(clippy::unwrap_used)]
#![warn(missing_docs)]

/// Configuration types for the diff algorithm.
pub mod config;

/// Core diff algorithm.
pub mod diff;

/// Data model for diff results.
pub mod model;

/// Rendering diff trees as human-readable output.
pub mod render;

pub use config::{
    AmbiguousMatchStrategy, ArrayMatchConfig, ArrayMatchMode, DiffConfig, MatchConfig,
};
pub use diff::{Error, diff};
pub use model::{DiffKind, DiffNode, DiffTree, PathSegment};
pub use render::DiffRenderer;
pub use render::yaml::YamlRenderer;
#[cfg(feature = "color")]
pub use render::yaml::ColorMode;
