use serde_json::Value;

use crate::model::{DiffKind, DiffNode, DiffTree, PathSegment};
use crate::render::{DiffRenderer, indicator};

/// Renders a `DiffTree` as YAML-like diff output.
///
/// Output uses unified diff conventions:
/// - ` ` prefix for unchanged lines (reserves column 0 for diff indicators)
/// - `-` prefix for expected (what we wanted but didn't get)
/// - `+` prefix for actual (what we got instead)
pub struct YamlRenderer {
    /// Maximum lines to render per side for large values.
    ///
    /// `None` means no truncation.
    pub max_lines_per_side: Option<u32>,

    /// Number of spaces per indentation level.
    pub indent_width: u16,
}

impl YamlRenderer {
    /// Default maximum lines to render per side.
    pub const DEFAULT_MAX_LINES_PER_SIDE: u32 = 20;

    /// Default number of spaces per indentation level.
    pub const DEFAULT_INDENT_WIDTH: u16 = 2;

    pub fn new() -> Self {
        Self {
            max_lines_per_side: Some(Self::DEFAULT_MAX_LINES_PER_SIDE),
            indent_width: Self::DEFAULT_INDENT_WIDTH,
        }
    }

    /// Sets the maximum lines to render per side.
    pub fn with_max_lines_per_side(mut self, max: Option<u32>) -> Self {
        self.max_lines_per_side = max;
        self
    }

    /// Sets the number of spaces per indentation level.
    pub fn with_indent_width(mut self, width: u16) -> Self {
        self.indent_width = width;
        self
    }

    /// Renders a single diff node at the given indentation depth.
    fn render_node(&self, node: &DiffNode, indent: u16, output: &mut String) {
        match node {
            DiffNode::Container {
                segment,
                omitted_count,
                children,
            } => {
                // Render the segment as a context line
                let label = format_segment_label(segment);
                push_line(output, indicator::UNCHANGED, indent, &format!("{label}:"));

                let child_indent = indent + self.indent_width;

                if *omitted_count > 0 {
                    let unit = omitted_unit(segment);
                    push_line(
                        output,
                        indicator::UNCHANGED,
                        child_indent,
                        &format!("# {omitted_count} {unit} omitted"),
                    );
                }

                for child in children {
                    render_child(self, child, indent, output);
                }
            }

            DiffNode::Leaf { segment, kind } => {
                render_leaf(self, segment, kind, indent, output);
            }
        }
    }
}

impl Default for YamlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffRenderer for YamlRenderer {
    fn render(&self, tree: &DiffTree) -> String {
        let mut output = String::new();
        for node in &tree.roots {
            // Root nodes start at indent level 0 (no leading spaces)
            self.render_node(node, 0, &mut output);
        }
        output
    }
}

/// Renders a child node, increasing indent by the configured width.
///
/// Array element segments (NamedElement, Index, Unmatched) get special
/// handling: the `- ` prefix is rendered on the context line, and children
/// are indented from there.
fn render_child(renderer: &YamlRenderer, node: &DiffNode, parent_indent: u16, output: &mut String) {
    let child_indent = parent_indent + renderer.indent_width;
    renderer.render_node(node, child_indent, output);
}

/// Renders a leaf node (a single difference).
fn render_leaf(
    renderer: &YamlRenderer,
    segment: &PathSegment,
    kind: &DiffKind,
    indent: u16,
    output: &mut String,
) {
    match kind {
        // Changed values are always scalars (compound types produce
        // Container nodes, not Leaf nodes). Safe to call format_scalar.
        DiffKind::Changed { actual, expected } => {
            let label = format_segment_label(segment);
            push_line(
                output,
                indicator::EXPECTED,
                indent,
                &format!("{label}: {val}", val = format_scalar(expected)),
            );
            push_line(
                output,
                indicator::ACTUAL,
                indent,
                &format!("{label}: {val}", val = format_scalar(actual)),
            );
        }

        DiffKind::Missing { expected } => {
            let label = format_segment_label(segment);
            if is_scalar(expected) {
                // Scalar missing value, safe to call format_scalar
                push_line(
                    output,
                    indicator::EXPECTED,
                    indent,
                    &format!("{label}: {val}", val = format_scalar(expected)),
                );
            } else {
                // Compound missing value. Render the key, then the full
                // expected value as `-` prefixed YAML lines.
                push_line(output, indicator::EXPECTED, indent, &format!("{label}:"));
                render_value_truncated(
                    output,
                    indicator::EXPECTED,
                    indent + renderer.indent_width,
                    renderer.indent_width,
                    expected,
                    renderer.max_lines_per_side,
                );
            }
        }

        DiffKind::TypeMismatch {
            actual,
            actual_type,
            expected,
            expected_type,
        } => {
            let label = format_segment_label(segment);

            // Build the content portion of each header line (before the comment)
            let expected_header = if is_scalar(expected) {
                format!("{label}: {val}", val = format_scalar(expected))
            } else {
                format!("{label}:")
            };
            let actual_header = if is_scalar(actual) {
                format!("{label}: {val}", val = format_scalar(actual))
            } else {
                format!("{label}:")
            };

            // Pad the shorter header so the type comments align
            let max_len = expected_header.len().max(actual_header.len());

            // Render expected side
            push_line(
                output,
                indicator::EXPECTED,
                indent,
                &format!("{expected_header:<width$} # expected: {expected_type}", width = max_len),
            );
            if !is_scalar(expected) {
                render_value_truncated(
                    output,
                    indicator::EXPECTED,
                    indent + renderer.indent_width,
                    renderer.indent_width,
                    expected,
                    renderer.max_lines_per_side,
                );
            }

            // Render actual side
            push_line(
                output,
                indicator::ACTUAL,
                indent,
                &format!("{actual_header:<width$} # actual: {actual_type}", width = max_len),
            );
            if !is_scalar(actual) {
                render_value_truncated(
                    output,
                    indicator::ACTUAL,
                    indent + renderer.indent_width,
                    renderer.indent_width,
                    actual,
                    renderer.max_lines_per_side,
                );
            }
        }
    }
}

/// Returns the appropriate unit word for omitted count based on the segment type.
///
/// Object keys use "fields", array segments use "items".
fn omitted_unit(segment: &PathSegment) -> &'static str {
    match segment {
        PathSegment::Key(_) => "fields",
        PathSegment::NamedElement { .. } | PathSegment::Index(_) | PathSegment::Unmatched => {
            "items"
        }
    }
}

/// Formats a path segment as a label for rendering.
fn format_segment_label(segment: &PathSegment) -> String {
    match segment {
        PathSegment::Key(key) => key.clone(),
        PathSegment::NamedElement {
            match_key,
            match_value,
        } => format!("- {match_key}: {match_value}"),
        PathSegment::Index(i) => format!("- # index {i}"),
        PathSegment::Unmatched => "-".to_owned(),
    }
}

/// Renders a compound value with optional line truncation.
///
/// Renders into a temporary buffer, then appends to `output`. If the
/// rendered output exceeds `max_lines`, only the first `max_lines` lines
/// are kept and a `# N more lines` marker is appended.
fn render_value_truncated(
    output: &mut String,
    prefix: char,
    indent: u16,
    indent_width: u16,
    value: &Value,
    max_lines: Option<u32>,
) {
    let mut buf = String::new();
    render_value(&mut buf, prefix, indent, indent_width, value);

    match max_lines {
        Some(max) => {
            let lines: Vec<&str> = buf.lines().collect();
            let total = lines.len() as u32;

            if total <= max {
                output.push_str(&buf);
            } else {
                // Append the first max_lines lines
                for line in &lines[..max as usize] {
                    output.push_str(line);
                    output.push('\n');
                }
                // Append the truncation marker with the same prefix
                let remaining = total - max;
                push_line(output, prefix, indent, &format!("# {remaining} more lines"));
            }
        }
        None => {
            output.push_str(&buf);
        }
    }
}

/// Renders a single key-value pair as YAML.
///
/// Scalars render as `key: value` on one line. Objects and arrays render
/// `key:` followed by the recursively rendered value on subsequent lines.
/// Also used for array element first keys via `render_array_element`,
/// where `key` is prefixed with `- ` (e.g. `- name`).
fn render_key_value(
    output: &mut String,
    prefix: char,
    indent: u16,
    indent_width: u16,
    key: &str,
    value: &Value,
) {
    if is_scalar(value) {
        push_line(
            output,
            prefix,
            indent,
            &format!("{key}: {val}", val = format_scalar(value)),
        );
    } else {
        push_line(output, prefix, indent, &format!("{key}:"));
        render_value(output, prefix, indent + indent_width, indent_width, value);
    }
}

/// Recursively renders a JSON value as YAML lines with the given prefix.
///
/// Used for rendering compound values in Missing and TypeMismatch diffs.
/// Each line is prefixed with the indicator character (e.g. `-` for expected).
fn render_value(
    output: &mut String,
    prefix: char,
    indent: u16,
    indent_width: u16,
    value: &Value,
) {
    match value {
        // Scalars render as a single value (caller handles the key)
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            push_line(output, prefix, indent, &format_scalar(value));
        }

        Value::Object(map) => {
            for (key, val) in map {
                render_key_value(output, prefix, indent, indent_width, key, val);
            }
        }

        Value::Array(arr) => {
            for elem in arr {
                if is_scalar(elem) {
                    push_line(
                        output,
                        prefix,
                        indent,
                        &format!("- {val}", val = format_scalar(elem)),
                    );
                } else {
                    // Render first key on the same line as `- `, rest indented
                    render_array_element(output, prefix, indent, indent_width, elem);
                }
            }
        }
    }
}

/// Renders a compound array element, placing the first object key on the
/// same line as the `- ` marker for natural YAML formatting.
fn render_array_element(
    output: &mut String,
    prefix: char,
    indent: u16,
    indent_width: u16,
    value: &Value,
) {
    match value {
        Value::Object(map) => {
            let mut first = true;
            for (key, val) in map {
                if first {
                    // First key goes on the `- ` line
                    render_key_value(output, prefix, indent, indent_width, &format!("- {key}"), val);
                    first = false;
                } else {
                    // Subsequent keys are indented past the `- `
                    render_key_value(output, prefix, indent + indent_width, indent_width, key, val);
                }
            }
        }

        // Non-object array elements (nested arrays)
        _ => {
            push_line(output, prefix, indent, "-");
            render_value(output, prefix, indent + indent_width, indent_width, value);
        }
    }
}

/// Pushes a single line to the output with the given prefix and indentation.
fn push_line(output: &mut String, prefix: char, indent: u16, content: &str) {
    output.push(prefix);
    for _ in 0..indent {
        output.push(' ');
    }
    output.push_str(content);
    output.push('\n');
}

/// Formats a JSON value as a YAML scalar.
fn format_scalar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if needs_yaml_quoting(s) {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{escaped}\"")
            } else {
                s.clone()
            }
        }
        Value::Array(_) | Value::Object(_) => {
            unreachable!("format_scalar called with compound value")
        }
    }
}

/// Returns true if a string needs quoting in YAML.
fn needs_yaml_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    // Values that YAML would interpret as non-strings
    const SPECIAL: &[&str] = &[
        "true", "false", "null", "yes", "no", "on", "off", "True", "False", "Null", "Yes", "No",
        "On", "Off", "TRUE", "FALSE", "NULL", "YES", "NO", "ON", "OFF",
    ];
    if SPECIAL.contains(&s) {
        return true;
    }

    // Strings that look like numbers
    if s.parse::<f64>().is_ok() {
        return true;
    }

    // Strings with special YAML characters
    s.contains(':')
        || s.contains('#')
        || s.contains('\n')
        || s.starts_with(' ')
        || s.ends_with(' ')
        || s.starts_with('{')
        || s.starts_with('[')
        || s.starts_with('*')
        || s.starts_with('&')
        || s.starts_with('!')
        || s.starts_with('|')
        || s.starts_with('>')
}

/// Returns true if a value is a scalar (not an object or array).
fn is_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DiffConfig, diff};
    use indoc::indoc;
    use serde_json::json;

    fn render(actual: &Value, expected: &Value) -> String {
        let config = DiffConfig::default();
        let tree = diff(actual, expected, &config);
        YamlRenderer::new().render(&tree)
    }

    #[test]
    fn scalar_changed_renders_minus_plus() {
        let output = render(
            &json!({"name": "actual_value"}),
            &json!({"name": "expected_value"}),
        );
        assert_eq!(
            output,
            indoc! {"
                -name: expected_value
                +name: actual_value
            "}
        );
    }

    #[test]
    fn nested_scalar_changed() {
        let output = render(
            &json!({"a": {"b": "actual"}}),
            &json!({"a": {"b": "expected"}}),
        );
        assert_eq!(
            output,
            indoc! {"
                 a:
                -  b: expected
                +  b: actual
            "}
        );
    }

    #[test]
    fn missing_scalar_key() {
        let output = render(&json!({"a": 1}), &json!({"a": 1, "b": 2}));
        assert_eq!(
            output,
            indoc! {"
                -b: 2
            "}
        );
    }

    #[test]
    fn equal_values_render_empty() {
        let output = render(&json!({"a": 1}), &json!({"a": 1}));
        assert_eq!(output, "");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn type_mismatch_scalar() {
        // actual has number 42, expected has string "42".
        // "42" is quoted because it looks like a number in YAML.
        // Comments are aligned by padding the shorter line.
        let output = render(&json!({"a": 42}), &json!({"a": "42"}));
        assert_eq!(
            output,
            indoc! {r#"
                -a: "42" # expected: string
                +a: 42   # actual: number
            "#}
        );
    }

    #[test]
    fn type_mismatch_null_vs_object() {
        let output = render(&json!({"a": null}), &json!({"a": {"b": 1}}));
        assert_eq!(
            output,
            indoc! {"
                -a:      # expected: object
                -  b: 1
                +a: null # actual: null
            "}
        );
    }

    #[test]
    fn missing_object_subtree() {
        let output = render(
            &json!({"a": 1}),
            &json!({"a": 1, "b": {"x": 1, "y": 2}}),
        );
        assert_eq!(
            output,
            indoc! {"
                -b:
                -  x: 1
                -  y: 2
            "}
        );
    }

    #[test]
    fn missing_array_subtree() {
        let output = render(
            &json!({"a": 1}),
            &json!({"a": 1, "items": [1, 2, 3]}),
        );
        assert_eq!(
            output,
            indoc! {"
                -items:
                -  - 1
                -  - 2
                -  - 3
            "}
        );
    }

    #[test]
    fn missing_nested_object_in_array() {
        let output = render(
            &json!({"a": 1}),
            &json!({"a": 1, "items": [{"name": "foo", "value": "bar"}]}),
        );
        assert_eq!(
            output,
            indoc! {"
                -items:
                -  - name: foo
                -    value: bar
            "}
        );
    }

    #[test]
    fn missing_subtree_truncated() {
        // Use a renderer with max 2 lines per side.
        // Keys are alphabetically ordered (serde_json uses BTreeMap).
        let config = DiffConfig::default();
        let actual = json!({"a": 1});
        let expected = json!({"a": 1, "b": {"p": 1, "q": 2, "r": 3, "s": 4}});
        let tree = diff(&actual, &expected, &config);
        let output = YamlRenderer::new()
            .with_max_lines_per_side(Some(2))
            .render(&tree);
        assert_eq!(
            output,
            indoc! {"
                -b:
                -  p: 1
                -  q: 2
                -  # 2 more lines
            "}
        );
    }

    #[test]
    fn truncation_disabled_renders_all_lines() {
        let config = DiffConfig::default();
        let actual = json!({"a": 1});
        let expected = json!({"a": 1, "b": {"x": 1, "y": 2, "z": 3}});
        let tree = diff(&actual, &expected, &config);
        let output = YamlRenderer::new()
            .with_max_lines_per_side(None)
            .render(&tree);
        assert_eq!(
            output,
            indoc! {"
                -b:
                -  x: 1
                -  y: 2
                -  z: 3
            "}
        );
    }

    #[test]
    fn omitted_fields_comment() {
        // inner object has 3 keys, expected checks 1 that differs. 2 fields omitted.
        let output = render(
            &json!({"outer": {"a": 1, "b": 2, "c": 3}}),
            &json!({"outer": {"a": 99}}),
        );
        assert_eq!(
            output,
            indoc! {"
                 outer:
                   # 2 fields omitted
                -  a: 99
                +  a: 1
            "}
        );
    }
}
