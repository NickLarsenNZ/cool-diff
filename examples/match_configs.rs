/// Demonstrates custom match configs with a mix of ordered and unordered arrays.
///
/// - `steps` uses index-based matching (order matters, like a pipeline).
/// - `tags` uses contains matching (just check presence, order irrelevant).
/// - `contributors` uses key-based matching on `email`.
fn main() {
    use cool_diff::{
        ArrayMatchConfig, ArrayMatchMode, DiffConfig, DiffRenderer as _, MatchConfig, YamlRenderer,
    };

    let actual = serde_json::json!({
        "project": "cool-diff",
        "steps": ["build", "test", "deploy"],
        "tags": ["rust", "diff", "yaml"],
        "contributors": [
            {"email": "alice@example.com", "role": "maintainer"},
            {"email": "bob@example.com", "role": "contributor"},
        ],
    });

    let expected = serde_json::json!({
        "project": "cool-diff",
        "steps": ["build", "lint", "deploy"],
        "tags": ["yaml"],
        "contributors": [
            {"email": "bob@example.com", "role": "reviewer"},
        ],
    });

    let config = DiffConfig {
        match_config: MatchConfig::new()
            // steps: order matters (index-based is the default, shown explicitly)
            .with_config_at("steps", ArrayMatchConfig::new(ArrayMatchMode::Index))
            // tags: just check presence
            .with_config_at("tags", ArrayMatchConfig::new(ArrayMatchMode::Contains))
            // contributors: match by email
            .with_config_at(
                "contributors",
                ArrayMatchConfig::new(ArrayMatchMode::Key("email".to_owned())),
            ),
        ..DiffConfig::default()
    };

    let tree = cool_diff::diff(&actual, &expected, &config).unwrap();
    let output = YamlRenderer::new().render(&tree);
    print!("{output}");
}
