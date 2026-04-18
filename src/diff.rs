use serde_json::Value;

use crate::config::DiffConfig;
use crate::model::{DiffKind, DiffNode, DiffTree};

/// Named constant to signify no differences were found.
const NO_DIFFERENCES: Vec<DiffNode> = vec![];

/// Computes a diff tree between `actual` and `expected` values.
///
/// The walk is driven by `expected`. Only paths present in the expected
/// value are compared. Fields in `actual` that have no corresponding
/// expected entry are counted as omitted but not diffed.
pub fn diff(actual: &Value, expected: &Value, config: &DiffConfig) -> DiffTree {
    // The root of the diff tree has an empty path
    let path = "";
    let roots = match diff_values(actual, expected, config, path) {
        // e.g. actual = 42, expected = 42
        // or actual = {...}, expected = {...}
        // or actual = [...], expected = [...]
        DiffResult::Equal => NO_DIFFERENCES,
        // TODO: handle root-level leaf diffs (e.g. actual = 42, expected = "hello")
        DiffResult::Leaf(_kind) => unimplemented!("root-level leaf diff"),
        // e.g. actual = {a: 1, b: 2}, expected = {a: 1, b: 3}
        DiffResult::Children { nodes, .. } => nodes,
    };
    DiffTree { roots }
}

/// The result of comparing two values. Separates "what kind of diff" from
/// node construction, since the caller provides the `PathSegment`.
enum DiffResult {
    /// Values are equal.
    Equal,

    /// A leaf-level difference (scalar mismatch or type mismatch).
    /// The caller wraps this in a `DiffNode::Leaf` with the appropriate segment.
    Leaf(DiffKind),

    /// Child diff nodes from comparing container contents (objects or arrays).
    /// The caller wraps this in a `DiffNode::Container` with the appropriate segment.
    Children {
        nodes: Vec<DiffNode>,
        omitted_count: u16,
    },
}

/// Recursively compares two values and returns a diff result.
///
/// `path` is the dot-separated path to the current position, used to look up
/// array match configuration.
fn diff_values(actual: &Value, expected: &Value, _config: &DiffConfig, _path: &str) -> DiffResult {
    // Type mismatch at the discriminant level (e.g. string vs number,
    // object vs array).
    if std::mem::discriminant(actual) != std::mem::discriminant(expected) {
        return DiffResult::Leaf(DiffKind::TypeMismatch {
            actual: actual.clone(),
            actual_type: value_type_name(actual),
            expected: expected.clone(),
            expected_type: value_type_name(expected),
        });
    }

    match (actual, expected) {
        // Scalars: direct comparison.
        (Value::Null, Value::Null) => DiffResult::Equal,
        (Value::Bool(a), Value::Bool(e)) if a == e => DiffResult::Equal,
        (Value::Number(a), Value::Number(e)) if a == e => DiffResult::Equal,
        (Value::String(a), Value::String(e)) if a == e => DiffResult::Equal,

        // Scalar mismatch (same type, different value).
        (Value::Bool(_), Value::Bool(_))
        | (Value::Number(_), Value::Number(_))
        | (Value::String(_), Value::String(_)) => DiffResult::Leaf(DiffKind::Changed {
            actual: actual.clone(),
            expected: expected.clone(),
        }),

        // TODO: object comparison
        (Value::Object(_), Value::Object(_)) => unimplemented!("object comparison"),

        // TODO: array comparison
        (Value::Array(_), Value::Array(_)) => unimplemented!("array comparison"),

        _ => unreachable!("discriminant check above ensures matching types"),
    }
}

/// Returns a human-readable type name for a JSON value.
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
