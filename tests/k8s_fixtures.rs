use std::path::PathBuf;

use cool_diff::{
    ArrayMatchConfig, ArrayMatchMode, DiffConfig, DiffRenderer, MatchConfig, YamlRenderer, diff,
};
use rstest::rstest;
use serde::Deserialize;

/// Standard Kubernetes config: match common array fields by name.
fn k8s_config() -> DiffConfig {
    DiffConfig::new().with_match_config(
        MatchConfig::new()
            .with_config_at(
                "spec.containers",
                ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
            )
            .with_config_at(
                "spec.containers.env",
                ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
            )
            .with_config_at(
                "spec.volumes",
                ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
            )
            .with_config_at(
                "spec.ports",
                ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
            )
            .with_config_at(
                "spec.template.spec.containers",
                ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
            )
            .with_config_at(
                "spec.template.spec.containers.env",
                ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
            ),
    )
}

/// Parses a multi-document YAML fixture file into (actual, expected) Values.
///
/// The first document is actual, the second is expected.
fn load_fixture(yaml: &str) -> (serde_json::Value, serde_json::Value) {
    let docs: Vec<serde_json::Value> = serde_yaml::Deserializer::from_str(yaml)
        .map(|doc| serde_json::Value::deserialize(doc).expect("failed to parse YAML document"))
        .collect();
    assert_eq!(docs.len(), 2, "fixture must have exactly 2 YAML documents");
    (docs[0].clone(), docs[1].clone())
}

#[rstest]
fn fixture_test(#[files("tests/fixtures/*.yaml")] path: PathBuf) {
    let yaml = std::fs::read_to_string(&path).expect("failed to read fixture file");
    let (actual, expected) = load_fixture(&yaml);

    let config = k8s_config();
    let tree = diff(&actual, &expected, &config).expect("diff with valid fixture inputs");
    let output = YamlRenderer::new()
        .with_max_lines_per_side(None)
        .render(&tree);

    // Load expected diff from companion .diff file
    let diff_path = path.with_extension("diff");
    let expected_diff = std::fs::read_to_string(&diff_path).unwrap_or_else(|_| {
        panic!(
            "missing expected diff file: {path}\n\
             Create it with the following content:\n\n\
             {output}",
            path = diff_path.display(),
        );
    });

    assert_eq!(output, expected_diff);
}
