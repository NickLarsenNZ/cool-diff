use serde_json::Value;

use crate::config::DiffConfig;
use crate::model::{DiffNode, DiffTree};

/// Named empty vec to signify no difference
const NO_DIFFERENCE: Vec<DiffNode> = vec![];

/// Computes a diff tree between `actual` and `expected` values.
///
/// The walk is driven by `expected`. Only paths present in the expected
/// value are compared. Fields in `actual` that have no corresponding
/// expected entry are counted as omitted but not diffed.
pub fn diff(actual: &Value, expected: &Value, config: &DiffConfig) -> DiffTree {
    // The root of the diff tree has an empty path
    let path = "";
    let roots = diff_values(actual, expected, config, path);
    DiffTree { roots }
}

/// Recursively compares two values and returns diff nodes for any differences.
///
/// `path` is the dot-separated path to the current position, used to look up
/// array match configuration.
fn diff_values(
    actual: &Value,
    expected: &Value,
    _config: &DiffConfig,
    _path: &str,
) -> Vec<DiffNode> {
    // Type mismatch at the discriminant level (e.g. string vs number,
    // object vs array). We don't recurse further in this case.
    if std::mem::discriminant(actual) != std::mem::discriminant(expected) {
        // Both null is equal, but null vs non-null is a type mismatch.
        // discriminant check already handles this since Null only matches Null.
        return NO_DIFFERENCE;
    }

    match (actual, expected) {
        // Scalars: direct comparison.
        (Value::Null, Value::Null) => NO_DIFFERENCE,
        (Value::Bool(a), Value::Bool(e)) if a == e => NO_DIFFERENCE,
        (Value::Number(a), Value::Number(e)) if a == e => NO_DIFFERENCE,
        (Value::String(a), Value::String(e)) if a == e => NO_DIFFERENCE,

        // Scalar mismatch (same type, different value).
        // TODO: return Changed node
        (Value::Bool(_), Value::Bool(_))
        | (Value::Number(_), Value::Number(_))
        | (Value::String(_), Value::String(_)) => unimplemented!("scalar Changed"),

        // TODO: object comparison
        (Value::Object(_), Value::Object(_)) => unimplemented!("object comparison"),

        // TODO: array comparison
        (Value::Array(_), Value::Array(_)) => unimplemented!("array comparison"),

        _ => unreachable!("discriminant check above ensures matching types"),
    }
}
