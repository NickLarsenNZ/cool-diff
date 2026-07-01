//! End-to-end: derive a `MatchConfig` from a Kubernetes OpenAPI schema, override
//! an atomic list, then diff a Pod. Exercises the same path as
//! `examples/openapi_pod.rs`, but runs under `cargo test --features experimental`.
#![cfg(feature = "experimental")]

use cool_diff::{
    ArrayMatchConfig, ArrayMatchMode, DiffConfig, DiffRenderer as _, MatchConfig, YamlRenderer,
    diff, k8s_schema,
};
use indoc::indoc;
use serde_json::{Value, json};

/// The trimmed schema this test pins behavior against. The `openapi_pod`
/// example renders the same fixture.
const POD_SCHEMA: &str = include_str!("fixtures/pod_schema.json");

/// Derives the Pod config from the schema, then overrides the atomic (but
/// order-insensitive) tolerations list to `Contains`.
fn pod_match_config() -> MatchConfig {
    let doc: Value = serde_json::from_str(POD_SCHEMA).expect("sample schema is valid JSON");
    let schemas = &doc["components"]["schemas"];
    let pod = &schemas["io.k8s.api.core.v1.Pod"];
    k8s_schema::match_config_from_schema(pod, Some(schemas)).with_config_at(
        "spec.tolerations",
        ArrayMatchConfig::new(ArrayMatchMode::Contains),
    )
}

#[test]
fn ports_use_the_derived_composite_key() {
    let config = pod_match_config();
    let mode = config
        .config_at("spec.containers.ports")
        .expect("ports configured from schema")
        .mode();
    assert!(matches!(mode, ArrayMatchMode::Key(keys)
        if keys == &["containerPort".to_owned(), "protocol".to_owned()]
    ));
}

#[test]
fn tolerations_overridden_to_contains() {
    let config = pod_match_config();
    let mode = config
        .config_at("spec.tolerations")
        .expect("tolerations overridden")
        .mode();
    assert!(matches!(mode, ArrayMatchMode::Contains));
}

#[test]
fn schema_derived_config_diffs_pod_end_to_end() {
    let actual = json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "my-pod" },
        "spec": {
            "containers": [
                {
                    "name": "app",
                    "image": "myapp:1.0",
                    "ports": [
                        { "containerPort": 53, "protocol": "UDP" },
                        { "containerPort": 53, "protocol": "TCP" }
                    ]
                }
            ],
            "tolerations": [
                { "key": "node.kubernetes.io/not-ready", "operator": "Exists", "effect": "NoExecute" },
                { "key": "node.kubernetes.io/unreachable", "operator": "Exists", "effect": "NoExecute" }
            ]
        }
    });
    // Asserts the 53/TCP port carries a name the actual lacks, and a toleration
    // that sits second in the actual list.
    let expected = json!({
        "spec": {
            "containers": [
                {
                    "name": "app",
                    "ports": [
                        { "containerPort": 53, "protocol": "TCP", "name": "dns-tcp" }
                    ]
                }
            ],
            "tolerations": [
                { "key": "node.kubernetes.io/unreachable", "operator": "Exists", "effect": "NoExecute" }
            ]
        }
    });

    let config = DiffConfig::new().with_match_config(pod_match_config());
    let tree = diff(&actual, &expected, &config).expect("diff with valid inputs");
    let output = YamlRenderer::new().render(&tree);

    // The composite key matches the 53/TCP port specifically, so the only
    // difference is the missing port name. The Contains override means the
    // second-position toleration matches without noise.
    assert_eq!(
        output,
        indoc! {"
             spec:
               containers:
                 - name: app
                   # 1 field omitted
                   ports:
                     # 1 item omitted
                     - containerPort: 53
                       protocol: TCP
            -          name: dns-tcp
        "}
    );
}
