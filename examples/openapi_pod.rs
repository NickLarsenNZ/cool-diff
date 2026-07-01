//! Derive a [`MatchConfig`] from a Kubernetes OpenAPI schema, then diff a Pod.
//!
//! Requires the `experimental` feature:
//!
//! ```sh
//! cargo run --example openapi_pod --features experimental
//! ```
//!
//! The array-matching rules are read straight from the schema's
//! `x-kubernetes-*` extensions, then one atomic list is overridden by hand to
//! show the escape hatch for order-insensitive lists.

use cool_diff::{
    ArrayMatchConfig, ArrayMatchMode, DiffConfig, DiffRenderer as _, MatchConfig, YamlRenderer,
    diff, k8s_schema,
};
use serde_json::{Value, json};

/// A trimmed OpenAPI v3 schema in the real Kubernetes layout: `$ref`s wrapped in
/// `allOf` and resolved against `components/schemas`. Owned by the end-to-end
/// test (`tests/k8s_schema_pod.rs`), which pins behavior against it.
const POD_SCHEMA: &str = include_str!("../tests/fixtures/pod_schema.json");

/// Builds the match config for a Pod: derive from the schema, then override the
/// one atomic list that is really order-insensitive.
fn pod_match_config() -> MatchConfig {
    let doc: Value = serde_json::from_str(POD_SCHEMA).expect("sample schema is valid JSON");
    let schemas = &doc["components"]["schemas"];
    let pod = &schemas["io.k8s.api.core.v1.Pod"];

    // Derived straight from the schema. This picks up spec.containers keyed by
    // name and spec.containers.ports keyed by the (containerPort, protocol) pair.
    let derived = k8s_schema::match_config_from_schema(pod, Some(schemas));

    // spec.tolerations is `atomic` in the schema, so it derives to index
    // matching. It is order-insensitive though (a pod tolerates a taint if any
    // toleration matches), so override it to Contains: match a toleration
    // anywhere, regardless of position.
    derived.with_config_at(
        "spec.tolerations",
        ArrayMatchConfig::new(ArrayMatchMode::Contains),
    )
}

/// The observed Pod. Two ports share containerPort 53 (UDP and TCP), and the
/// tolerations are in a particular order.
fn actual_pod() -> Value {
    json!({
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
    })
}

/// The expected Pod. Asserts the 53/TCP port carries a name (which the actual
/// lacks), and asserts a toleration that sits second in the actual list.
fn expected_pod() -> Value {
    json!({
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
    })
}

/// Diffs the two Pods with the derived + overridden config and renders the result.
fn render_diff() -> String {
    let config = DiffConfig::new().with_match_config(pod_match_config());
    let tree = diff(&actual_pod(), &expected_pod(), &config).expect("diff with valid inputs");
    YamlRenderer::new().render(&tree)
}

fn main() {
    let output = render_diff();
    if output.is_empty() {
        println!("Pod matches expected state.");
    } else {
        print!("{output}");
    }
}
