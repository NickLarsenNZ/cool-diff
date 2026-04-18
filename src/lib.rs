pub mod config;
pub mod diff;
pub mod model;

pub use config::{
    AmbiguousMatchStrategy, ArrayMatchConfig, ArrayMatchMode, DiffConfig, MatchConfig,
};
pub use diff::diff;
pub use model::{DiffKind, DiffNode, DiffTree, PathSegment};
