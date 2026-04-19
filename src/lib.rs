#![deny(clippy::unwrap_used)]

pub mod config;
pub mod diff;
pub mod model;
pub mod render;

pub use config::{
    AmbiguousMatchStrategy, ArrayMatchConfig, ArrayMatchMode, DiffConfig, MatchConfig,
};
pub use diff::{Error, diff};
pub use model::{DiffKind, DiffNode, DiffTree, PathSegment};
pub use render::DiffRenderer;
pub use render::yaml::YamlRenderer;
