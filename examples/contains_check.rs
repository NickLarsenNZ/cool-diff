use cool_diff::{
    AmbiguousMatchStrategy, ArrayMatchConfig, ArrayMatchMode, DiffConfig, DiffRenderer as _,
    MatchConfig, YamlRenderer,
};

/// Demonstrates checking that certain elements exist in an array,
/// regardless of order or extra elements.
///
/// Uses Contains mode with BestMatch to tolerate multiple matches
/// without failing.
fn main() {
    let actual = serde_json::json!({
        "inventory": [
            {"item": "apple", "count": 5},
            {"item": "banana", "count": 12},
            {"item": "cherry", "count": 3},
            {"item": "date", "count": 8},
        ],
    });

    // We want to verify that specific items exist with exact field values.
    // Contains mode checks that the expected fields are a subset of some
    // actual element.
    let expected = serde_json::json!({
        "inventory": [
            {"item": "banana", "count": 12},
            {"item": "grape", "count": 1},
        ],
    });

    let config = DiffConfig::new().with_match_config(
        MatchConfig::new().with_config_at(
            "inventory",
            ArrayMatchConfig::new(ArrayMatchMode::Contains)
                .with_ambiguous_strategy(AmbiguousMatchStrategy::BestMatch),
        ),
    );

    let tree = cool_diff::diff(&actual, &expected, &config).unwrap();

    if tree.is_empty() {
        println!("All expected items found.");
    } else {
        println!("Some expected items are missing or differ:");
        let output = YamlRenderer::new().render(&tree);
        print!("{output}");
    }
}
