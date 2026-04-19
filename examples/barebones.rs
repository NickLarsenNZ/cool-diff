use cool_diff::DiffRenderer as _;

/// Simplest possible usage with all defaults.
///
/// Uses index-based array matching and the default renderer settings.
fn main() {
    let actual = serde_json::json!({
        "name": "Alice",
        "age": 30,
        "active": true,
    });

    let expected = serde_json::json!({
        "name": "Alice",
        "age": 25,
        "active": false,
    });

    let config = cool_diff::DiffConfig::default();
    let tree = cool_diff::diff(&actual, &expected, &config);

    if tree.is_empty() {
        println!("No differences found.");
    } else {
        let output = cool_diff::YamlRenderer::new().render(&tree);
        print!("{output}");
    }
}
