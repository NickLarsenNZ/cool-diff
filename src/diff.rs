use serde_json::Value;

use crate::config::DiffConfig;
use crate::model::{DiffKind, DiffNode, DiffTree, PathSegment};

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
fn diff_values(actual: &Value, expected: &Value, config: &DiffConfig, path: &str) -> DiffResult {
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

        // object comparison
        (Value::Object(actual_map), Value::Object(expected_map)) => {
            diff_objects(actual_map, expected_map, config, path)
        }

        // TODO: array comparison
        (Value::Array(_), Value::Array(_)) => unimplemented!("array comparison"),

        _ => unreachable!("discriminant check above ensures matching types"),
    }
}

/// Compares two objects and returns a diff result.
///
/// Iterates expected keys. For each key:
/// - Missing from actual: produces a `Missing` leaf.
/// - Present in actual: recurses via `diff_values` and wraps the result.
///
/// `omitted_count` tracks actual keys not present in expected.
fn diff_objects(
    actual_map: &serde_json::Map<String, Value>,
    expected_map: &serde_json::Map<String, Value>,
    config: &DiffConfig,
    path: &str,
) -> DiffResult {
    let mut children = Vec::new();

    // Loop through the expected map pairs and then check each against the
    // actual map for the same key.
    for (key, expected_val) in expected_map {
        // Build the dot-separated path for config lookups (e.g. "spec.containers").
        // At the root level, path is empty so we avoid a leading dot.
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        let segment = PathSegment::Key(key.clone());

        match actual_map.get(key) {
            // Expected key doesn't exist in actual
            None => {
                children.push(DiffNode::Leaf {
                    segment,
                    kind: DiffKind::Missing {
                        expected: expected_val.clone(),
                    },
                });
            }

            // Key exists in both, recurse to compare values
            Some(actual_val) => {
                match diff_values(actual_val, expected_val, config, &child_path) {
                    // Values are equal, nothing to record
                    DiffResult::Equal => {}

                    // Scalar or type mismatch, wrap as a leaf node
                    DiffResult::Leaf(kind) => {
                        children.push(DiffNode::Leaf { segment, kind });
                    }

                    // Nested differences in a child object or array
                    DiffResult::Children {
                        nodes,
                        omitted_count,
                    } => {
                        children.push(DiffNode::Container {
                            segment,
                            omitted_count,
                            children: nodes,
                        });
                    }
                }
            }
        }
    }

    // no differences
    if children.is_empty() {
        return DiffResult::Equal;
    }

    // Count of actual keys not checked because they have no corresponding
    // expected key. The renderer uses this for "# N fields omitted" markers.
    let omitted_count = actual_map.len().saturating_sub(expected_map.len()) as u16;
    DiffResult::Children {
        nodes: children,
        omitted_count,
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
