/// Demonstrates implementing a custom `DiffRenderer`.
///
/// This example builds a simple plain-text renderer that outputs one line
/// per difference, showing the full path and the kind of change.
use cool_diff::{DiffKind, DiffNode, DiffRenderer, DiffTree, PathSegment};

struct PlainTextRenderer;

impl DiffRenderer for PlainTextRenderer {
    fn render(&self, tree: &DiffTree) -> String {
        let mut output = String::new();
        for node in &tree.roots {
            render_node(node, &mut Vec::new(), &mut output);
        }
        output
    }
}

fn render_node(node: &DiffNode, path: &mut Vec<String>, output: &mut String) {
    match node {
        DiffNode::Container {
            segment, children, ..
        } => {
            path.push(format_segment(segment));
            for child in children {
                render_node(child, path, output);
            }
            path.pop();
        }

        DiffNode::Leaf { segment, kind } => {
            path.push(format_segment(segment));
            let full_path = path.join(".");

            match kind {
                DiffKind::Changed { actual, expected } => {
                    output.push_str(&format!(
                        "Changed: {full_path} - expected: {expected}, actual: {actual}\n"
                    ));
                }
                DiffKind::Missing { expected } => {
                    output.push_str(&format!("Missing: {full_path} - expected: {expected}\n"));
                }
                DiffKind::TypeMismatch {
                    actual_type,
                    expected_type,
                    ..
                } => {
                    output.push_str(&format!(
                        "TypeMismatch: {full_path} - expected: {expected_type}, actual: {actual_type}\n"
                    ));
                }
            }

            path.pop();
        }
    }
}

fn format_segment(segment: &PathSegment) -> String {
    match segment {
        PathSegment::Key(key) => key.clone(),
        PathSegment::NamedElement {
            match_key,
            match_value,
        } => format!("[{match_key}={match_value}]"),
        PathSegment::Index(i) => format!("[{i}]"),
        PathSegment::Unmatched => "[?]".to_owned(),
    }
}

fn main() {
    let actual = serde_json::json!({
        "name": "my-app",
        "version": "1.0",
        "settings": {
            "debug": true,
            "timeout": 30,
        },
    });

    let expected = serde_json::json!({
        "name": "my-app",
        "version": "2.0",
        "settings": {
            "debug": false,
            "log_level": "info",
        },
    });

    let config = cool_diff::DiffConfig::default();
    let tree = cool_diff::diff(&actual, &expected, &config).unwrap();
    let output = PlainTextRenderer.render(&tree);
    print!("{output}");
}
