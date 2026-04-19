use cool_diff::{
    AmbiguousMatchStrategy, ArrayMatchConfig, ArrayMatchMode, DiffConfig, DiffRenderer as _,
    MatchConfig, YamlRenderer,
};

/// Demonstrates strict ambiguity handling.
///
/// With `Strict` strategy (the default), the diff returns an error if
/// multiple actual elements could match a single expected element.
/// This forces the user to provide enough fields to disambiguate.
fn main() {
    let actual = serde_json::json!({
        "users": [
            {"name": "Alice", "role": "admin"},
            {"name": "Alice", "role": "user"},
            {"name": "Bob", "role": "user"},
        ],
    });

    // This matches uniquely since only one element has name "Bob"
    let expected_unique = serde_json::json!({
        "users": [
            {"name": "Bob", "role": "admin"},
        ],
    });

    // This is ambiguous since two elements have name "Alice"
    let expected_ambiguous = serde_json::json!({
        "users": [
            {"name": "Alice", "role": "admin"},
        ],
    });

    let config = DiffConfig {
        match_config: MatchConfig::new().with_config_at(
            "users",
            ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned()))
                // Strict is the default, but we set it explicitly here for clarity
                .with_ambiguous_strategy(AmbiguousMatchStrategy::Strict),
        ),
        ..DiffConfig::default()
    };

    // Unique match works fine
    match cool_diff::diff(&actual, &expected_unique, &config) {
        Ok(tree) => {
            println!("Unique match (Bob):");
            let output = YamlRenderer::new().render(&tree);
            print!("{output}");
        }
        Err(err) => println!("Error: {err}"),
    }

    println!();

    // Ambiguous match returns an error with Strict strategy
    match cool_diff::diff(&actual, &expected_ambiguous, &config) {
        Ok(tree) => {
            println!("No error (unexpected):");
            let output = YamlRenderer::new().render(&tree);
            print!("{output}");
        }
        Err(err) => println!("Ambiguous match (Alice): {err}"),
    }
}
