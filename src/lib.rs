pub mod config;
pub mod model;

pub use config::{AmbiguousMatchStrategy, DiffConfig, MatchConfig};
pub use model::{DiffKind, DiffNode, DiffTree, PathSegment};
