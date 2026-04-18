use serde_json::Value;

use crate::config::{ArrayMatchMode, DiffConfig};
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
        return DiffResult::Leaf(DiffKind::type_mismatch(
            actual.clone(),
            value_type_name(actual),
            expected.clone(),
            value_type_name(expected),
        ));
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
        | (Value::String(_), Value::String(_)) => {
            DiffResult::Leaf(DiffKind::changed(actual.clone(), expected.clone()))
        }

        // object comparison
        (Value::Object(actual_map), Value::Object(expected_map)) => {
            diff_objects(actual_map, expected_map, config, path)
        }

        // Array comparison, dispatched by match mode
        (Value::Array(actual_arr), Value::Array(expected_arr)) => {
            diff_arrays(actual_arr, expected_arr, config, path)
        }

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
                let kind = DiffKind::missing(expected_val.clone());
                children.push(DiffNode::leaf(segment, kind));
            }

            // Key exists in both, recurse to compare values
            Some(actual_val) => {
                match diff_values(actual_val, expected_val, config, &child_path) {
                    // Values are equal, nothing to record
                    DiffResult::Equal => {}

                    // Scalar or type mismatch, wrap as a leaf node
                    DiffResult::Leaf(kind) => {
                        children.push(DiffNode::leaf(segment, kind));
                    }

                    // Nested differences in a child object or array
                    DiffResult::Children {
                        nodes,
                        omitted_count,
                    } => {
                        children.push(DiffNode::container(segment, omitted_count, nodes));
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

/// Compares two arrays and returns a diff result.
///
/// Looks up the `ArrayMatchMode` for the current path and dispatches
/// to the appropriate matching strategy.
fn diff_arrays(
    actual_arr: &[Value],
    expected_arr: &[Value],
    config: &DiffConfig,
    path: &str,
) -> DiffResult {
    let mode = config
        .match_config
        .config_at(path)
        .map(|c| &c.mode)
        .unwrap_or(&config.default_array_mode);

    match mode {
        ArrayMatchMode::Index => diff_arrays_by_index(actual_arr, expected_arr, config, path),
        // TODO: key-based matching
        ArrayMatchMode::Key(_) => unimplemented!("key-based array matching"),
        // TODO: contains matching
        ArrayMatchMode::Contains => unimplemented!("contains array matching"),
    }
}

/// Index-based array matching. Compares elements at the same position.
///
/// For each expected element, if the actual array has an element at that
/// index, recurse. Otherwise, produce a `Missing` leaf.
fn diff_arrays_by_index(
    actual_arr: &[Value],
    expected_arr: &[Value],
    config: &DiffConfig,
    path: &str,
) -> DiffResult {
    let mut children = Vec::new();

    // Loop through the expected array items and then check each against the
    // actual array for the element of the same index.
    for (i, expected_elem) in expected_arr.iter().enumerate() {
        let segment = PathSegment::Index(i as u16);

        match actual_arr.get(i) {
            // Expected index is beyond the actual array length
            None => {
                let kind = DiffKind::missing(expected_elem.clone());
                children.push(DiffNode::leaf(segment, kind));
            }

            // Both sides have an element at this index, recurse
            Some(actual_elem) => {
                match diff_values(actual_elem, expected_elem, config, path) {
                    // Values are equal, nothing to record
                    DiffResult::Equal => {}

                    // Scalar or type mismatch, wrap as a leaf node
                    DiffResult::Leaf(kind) => {
                        children.push(DiffNode::leaf(segment, kind));
                    }

                    // Nested differences in a child object or array
                    DiffResult::Children {
                        nodes,
                        omitted_count,
                    } => {
                        children.push(DiffNode::container(segment, omitted_count, nodes));
                    }
                }
            }
        }
    }

    // no differences
    if children.is_empty() {
        return DiffResult::Equal;
    }

    // Extra elements in actual that have no corresponding expected element.
    // The renderer uses this for "# N items omitted" markers.
    let omitted_count = actual_arr.len().saturating_sub(expected_arr.len()) as u16;
    DiffResult::Children {
        nodes: children,
        omitted_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_config() -> DiffConfig {
        DiffConfig::default()
    }

    #[test]
    fn equal_objects_produce_empty_diff() {
        let actual = json!({"a": 1, "b": "hello"});
        let expected = json!({"a": 1, "b": "hello"});
        let tree = diff(&actual, &expected, &default_config());
        assert!(tree.is_empty());
    }

    #[test]
    fn scalar_changed() {
        let actual = json!({"a": {"b": {"c": "foo"}}});
        let expected = json!({"a": {"b": {"c": "bar"}}});
        let tree = diff(&actual, &expected, &default_config());

        // Should produce: a -> b -> c: Changed("foo" -> "bar")
        assert_eq!(tree.roots.len(), 1);
        let DiffNode::Container { segment, children, .. } = &tree.roots[0] else {
            panic!("expected Container");
        };
        assert!(matches!(segment, PathSegment::Key(k) if k == "a"));

        let DiffNode::Container { segment, children, .. } = &children[0] else {
            panic!("expected Container");
        };
        assert!(matches!(segment, PathSegment::Key(k) if k == "b"));

        let DiffNode::Leaf { segment, kind } = &children[0] else {
            panic!("expected Leaf");
        };
        assert!(matches!(segment, PathSegment::Key(k) if k == "c"));
        assert!(matches!(kind, DiffKind::Changed { actual, expected }
            if actual == &json!("foo") && expected == &json!("bar")
        ));
    }

    #[test]
    fn missing_key() {
        let actual = json!({"a": 1});
        let expected = json!({"a": 1, "b": 2});
        let tree = diff(&actual, &expected, &default_config());

        assert_eq!(tree.roots.len(), 1);
        let DiffNode::Leaf { segment, kind } = &tree.roots[0] else {
            panic!("expected Leaf");
        };
        assert!(matches!(segment, PathSegment::Key(k) if k == "b"));
        assert!(matches!(kind, DiffKind::Missing { expected } if expected == &json!(2)));
    }

    #[test]
    fn type_mismatch() {
        let actual = json!({"a": 42});
        let expected = json!({"a": "42"});
        let tree = diff(&actual, &expected, &default_config());

        assert_eq!(tree.roots.len(), 1);
        let DiffNode::Leaf { segment, kind } = &tree.roots[0] else {
            panic!("expected Leaf");
        };
        assert!(matches!(segment, PathSegment::Key(k) if k == "a"));
        assert!(matches!(kind, DiffKind::TypeMismatch {
            actual_type: "number",
            expected_type: "string",
            ..
        }));
    }

    #[test]
    fn omitted_count_reflects_extra_actual_keys() {
        let actual = json!({"a": 1, "b": 2, "c": 3});
        let expected = json!({"a": 99});
        let tree = diff(&actual, &expected, &default_config());

        assert_eq!(tree.roots.len(), 1);
        let DiffNode::Leaf { kind, .. } = &tree.roots[0] else {
            panic!("expected Leaf for Changed");
        };
        assert!(matches!(kind, DiffKind::Changed { .. }));

        // The root-level Children omitted_count should be 2 (b and c not in expected).
        // But since roots are unwrapped from Children, we need to check via diff_values directly.
        let result = diff_values(
            &actual,
            &expected,
            &default_config(),
            "",
        );
        assert!(matches!(result, DiffResult::Children { omitted_count: 2, .. }));
    }

    #[test]
    fn nested_missing_key() {
        let actual = json!({"a": {"x": 1}});
        let expected = json!({"a": {"x": 1, "y": 2}});
        let tree = diff(&actual, &expected, &default_config());

        assert_eq!(tree.roots.len(), 1);
        let DiffNode::Container { segment, children, omitted_count } = &tree.roots[0] else {
            panic!("expected Container");
        };
        assert!(matches!(segment, PathSegment::Key(k) if k == "a"));
        assert_eq!(*omitted_count, 0);

        assert_eq!(children.len(), 1);
        let DiffNode::Leaf { segment, kind } = &children[0] else {
            panic!("expected Leaf");
        };
        assert!(matches!(segment, PathSegment::Key(k) if k == "y"));
        assert!(matches!(kind, DiffKind::Missing { expected } if expected == &json!(2)));
    }

    #[test]
    fn index_based_array_equal() {
        let actual = json!({"items": [1, 2, 3]});
        let expected = json!({"items": [1, 2, 3]});
        let tree = diff(&actual, &expected, &default_config());
        assert!(tree.is_empty());
    }

    #[test]
    fn index_based_array_changed() {
        let actual = json!({"items": [1, 2, 3]});
        let expected = json!({"items": [1, 99, 3]});
        let tree = diff(&actual, &expected, &default_config());

        // items -> Index(1): Changed(2 -> 99)
        assert_eq!(tree.roots.len(), 1);
        let DiffNode::Container { children, .. } = &tree.roots[0] else {
            panic!("expected Container");
        };
        assert_eq!(children.len(), 1);
        let DiffNode::Leaf { segment, kind } = &children[0] else {
            panic!("expected Leaf");
        };
        assert!(matches!(segment, PathSegment::Index(1)));
        assert!(matches!(kind, DiffKind::Changed { actual, expected }
            if actual == &json!(2) && expected == &json!(99)
        ));
    }

    #[test]
    fn index_based_array_missing_element() {
        let actual = json!({"items": [1]});
        let expected = json!({"items": [1, 2, 3]});
        let tree = diff(&actual, &expected, &default_config());

        let DiffNode::Container { children, .. } = &tree.roots[0] else {
            panic!("expected Container");
        };
        assert_eq!(children.len(), 2);

        // Index 1 is missing
        let DiffNode::Leaf { segment, kind } = &children[0] else {
            panic!("expected Leaf");
        };
        assert!(matches!(segment, PathSegment::Index(1)));
        assert!(matches!(kind, DiffKind::Missing { expected } if expected == &json!(2)));

        // Index 2 is missing
        let DiffNode::Leaf { segment, kind } = &children[1] else {
            panic!("expected Leaf");
        };
        assert!(matches!(segment, PathSegment::Index(2)));
        assert!(matches!(kind, DiffKind::Missing { expected } if expected == &json!(3)));
    }

    #[test]
    fn index_based_array_omitted_count() {
        // actual has 5 elements, expected checks 2. Omitted count = 3.
        let actual = json!({"items": [1, 2, 3, 4, 5]});
        let expected = json!({"items": [1, 99]});
        let tree = diff(&actual, &expected, &default_config());

        // Root: items Container (omitted_count=0 since both objects have 1 key)
        let DiffNode::Container {
            segment,
            children,
            omitted_count,
        } = &tree.roots[0]
        else {
            panic!("expected Container for items key");
        };
        assert!(matches!(segment, PathSegment::Key(k) if k == "items"));
        assert_eq!(*omitted_count, 3);
        // Only one child: Index(1) Changed(2 -> 99). Index(0) is equal.
        assert_eq!(children.len(), 1);
        assert!(matches!(
            &children[0],
            DiffNode::Leaf {
                segment: PathSegment::Index(1),
                kind: DiffKind::Changed { .. },
            }
        ));
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
