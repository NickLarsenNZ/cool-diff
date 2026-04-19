/// Demonstrates diffing Kubernetes objects with key-based array matching.
///
/// Kubernetes resources use `name` as the distinguished key for most array
/// fields (containers, env vars, volumes, ports). This example shows how
/// to configure that.
fn main() {
    use cool_diff::{
        ArrayMatchConfig, ArrayMatchMode, DiffConfig, DiffRenderer as _, MatchConfig, YamlRenderer,
    };

    let actual: serde_json::Value = serde_json::from_str(
        r#"{
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": { "name": "my-pod" },
            "spec": {
                "containers": [
                    {
                        "name": "app",
                        "image": "myapp:1.0",
                        "env": [
                            { "name": "LOG_LEVEL", "value": "info" },
                            { "name": "PORT", "value": "8080" }
                        ],
                        "ports": [
                            { "containerPort": 8080, "name": "http" }
                        ]
                    },
                    {
                        "name": "sidecar",
                        "image": "proxy:2.0"
                    }
                ],
                "volumes": [
                    { "name": "config", "configMap": { "name": "my-config" } }
                ]
            }
        }"#,
    )
    .expect("failed to parse actual JSON");

    let expected: serde_json::Value = serde_json::from_str(
        r#"{
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": { "name": "my-pod" },
            "spec": {
                "containers": [
                    {
                        "name": "app",
                        "image": "myapp:2.0",
                        "env": [
                            { "name": "LOG_LEVEL", "value": "debug" }
                        ]
                    }
                ]
            }
        }"#,
    )
    .expect("failed to parse expected JSON");

    // In future, pre-configured MatchConfigs for common Kubernetes types
    // will be provided behind a "kubernetes" feature gate
    // (e.g. MatchConfig::kubernetes_from_value(&actual)).
    let pod_match_config = MatchConfig::new()
        .with_config_at(
            "spec.containers",
            ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
        )
        .with_config_at(
            "spec.containers.env",
            ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
        )
        .with_config_at(
            "spec.containers.ports",
            ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
        )
        .with_config_at(
            "spec.volumes",
            ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
        );

    let config = DiffConfig {
        match_config: pod_match_config,
        ..DiffConfig::default()
    };

    let tree = cool_diff::diff(&actual, &expected, &config).unwrap();

    if tree.is_empty() {
        println!("Pod matches expected state.");
    } else {
        println!("Pod differs from expected:");
        let output = YamlRenderer::new().render(&tree);
        print!("{output}");
    }
}
