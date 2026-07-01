//! Derive a [`MatchConfig`] from a Kubernetes OpenAPI schema.
//!
//! Kubernetes encodes array-matching semantics via vendor extensions on each
//! array property:
//!
//! - `x-kubernetes-list-type: map` + `x-kubernetes-list-map-keys: [fields]` maps
//!   to [`ArrayMatchMode::keys`] (the distinguished keys).
//! - `x-kubernetes-list-type: set` maps to [`ArrayMatchMode::Contains`].
//! - `x-kubernetes-list-type: atomic` (or absent) is index matching, the
//!   default, so it is omitted from the produced config.
//! - The legacy `x-kubernetes-patch-strategy: merge` +
//!   `x-kubernetes-patch-merge-key: k` maps to [`ArrayMatchMode::key`].
//!
//! This module is experimental and its API may change.

use std::collections::HashSet;

use serde_json::Value;

use crate::config::{ArrayMatchConfig, ArrayMatchMode, MatchConfig};

/// The `$ref` prefix for schemas in an OpenAPI v3 document
/// (e.g. `#/components/schemas/io.k8s.api.core.v1.Pod`).
const REF_PREFIX: &str = "#/components/schemas/";

/// Derives a [`MatchConfig`] from a Kubernetes OpenAPI object schema.
///
/// Recursively walks `root`, recording a per-path match mode for every array
/// whose vendor extensions call for something other than index matching. Paths
/// are dot-separated (e.g. `spec.containers.ports`), matching how the diff
/// engine looks up config for nested arrays.
///
/// `components` is the `#/components/schemas` map used to resolve `$ref`s. Pass
/// `None` for a fully-inlined schema (such as a CRD's `openAPIV3Schema`), where
/// there are no references to resolve.
pub fn match_config_from_schema(root: &Value, components: Option<&Value>) -> MatchConfig {
    let mut entries: Vec<(String, ArrayMatchMode)> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    collect(root, "", components, &mut entries, &mut visited);

    entries
        .into_iter()
        .fold(MatchConfig::new(), |config, (path, mode)| {
            config.with_config_at(&path, ArrayMatchConfig::new(mode))
        })
}

/// Recursively collects `(path, mode)` entries from a schema.
///
/// `visited` tracks the `$ref`s on the current descent so a self-referential
/// schema terminates. It is added on entry and removed on exit, so a shared
/// type referenced from sibling branches is still walked in each.
fn collect(
    schema: &Value,
    path: &str,
    components: Option<&Value>,
    entries: &mut Vec<(String, ArrayMatchMode)>,
    visited: &mut HashSet<String>,
) {
    // Resolve a `$ref` before anything else.
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if visited.contains(reference) {
            return; // cycle: stop descending this branch
        }
        if let Some(resolved) = resolve(reference, components) {
            visited.insert(reference.to_owned());
            collect(resolved, path, components, entries, visited);
            visited.remove(reference);
        }
        return;
    }

    // The Kubernetes spec wraps `$ref`s in an `allOf` alongside metadata
    // (`default`, `description`). Descend into each subschema at the same path.
    if let Some(subschemas) = schema.get("allOf").and_then(Value::as_array) {
        for subschema in subschemas {
            collect(subschema, path, components, entries, visited);
        }
        return;
    }

    if is_array(schema) {
        if let Some(mode) = array_match_mode(schema) {
            entries.push((path.to_owned(), mode));
        }
        // Recurse into the element schema at the same path, so nested arrays
        // (e.g. containers -> ports) are discovered under the array's path.
        if let Some(items) = schema.get("items") {
            collect(items, path, components, entries, visited);
        }
        return;
    }

    // Object schema: recurse into each property, extending the dot-path.
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (name, child) in properties {
            let child_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            collect(child, &child_path, components, entries, visited);
        }
    }
}

/// Resolves a `#/components/schemas/NAME` reference against the components map.
///
/// Returns `None` when there is no components map, or for references that do
/// not point into it (e.g. external `$ref`s), so they are simply not descended.
fn resolve<'a>(reference: &str, components: Option<&'a Value>) -> Option<&'a Value> {
    let name = reference.strip_prefix(REF_PREFIX)?;
    components?.get(name)
}

/// Returns true if the schema describes an array.
fn is_array(schema: &Value) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("array") || schema.get("items").is_some()
}

/// Determines the [`ArrayMatchMode`] for an array property schema from its
/// vendor extensions, or `None` when index matching (the default) applies.
fn array_match_mode(schema: &Value) -> Option<ArrayMatchMode> {
    // The newer `x-kubernetes-list-type` takes precedence over the legacy
    // patch-merge annotations.
    if let Some(list_type) = schema.get("x-kubernetes-list-type").and_then(Value::as_str) {
        return match list_type {
            "map" => {
                let keys: Vec<&str> = schema
                    .get("x-kubernetes-list-map-keys")
                    .and_then(Value::as_array)
                    .map(|keys| keys.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                // A `map` with no keys would produce an empty key set, which the
                // diff algorithm rejects. Omit it rather than emit a broken config.
                if keys.is_empty() {
                    None
                } else {
                    Some(ArrayMatchMode::keys(keys))
                }
            }
            "set" => Some(ArrayMatchMode::Contains),
            // "atomic" or anything unrecognised: index matching (the default).
            _ => None,
        };
    }

    legacy_merge_key(schema).map(ArrayMatchMode::key)
}

/// Extracts the legacy `x-kubernetes-patch-merge-key` when the patch strategy
/// is a merge.
fn legacy_merge_key(schema: &Value) -> Option<&str> {
    let strategy = schema
        .get("x-kubernetes-patch-strategy")
        .and_then(Value::as_str)?;
    // The strategy can be comma-separated, e.g. "merge,retainKeys".
    if !strategy.split(',').any(|part| part == "merge") {
        return None;
    }
    schema
        .get("x-kubernetes-patch-merge-key")
        .and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn list_type_map_becomes_composite_keys() {
        let schema = json!({
            "properties": {
                "ports": {
                    "type": "array",
                    "x-kubernetes-list-type": "map",
                    "x-kubernetes-list-map-keys": ["containerPort", "protocol"],
                    "items": {}
                }
            }
        });
        let config = match_config_from_schema(&schema, None);
        let mode = config.config_at("ports").expect("ports configured").mode();
        assert!(matches!(mode, ArrayMatchMode::Key(keys)
            if keys == &["containerPort".to_owned(), "protocol".to_owned()]
        ));
    }

    #[test]
    fn list_type_set_becomes_contains() {
        let schema = json!({
            "properties": {
                "finalizers": {
                    "type": "array",
                    "x-kubernetes-list-type": "set",
                    "items": {}
                }
            }
        });
        let config = match_config_from_schema(&schema, None);
        let mode = config
            .config_at("finalizers")
            .expect("finalizers configured")
            .mode();
        assert!(matches!(mode, ArrayMatchMode::Contains));
    }

    #[test]
    fn atomic_and_absent_list_types_are_omitted() {
        let schema = json!({
            "properties": {
                "args": {
                    "type": "array",
                    "x-kubernetes-list-type": "atomic",
                    "items": {}
                },
                "command": { "type": "array", "items": {} }
            }
        });
        let config = match_config_from_schema(&schema, None);
        // Omission is intentional, not an oversight: atomic and unannotated
        // arrays use index matching, which is MatchConfig's default. Emitting a
        // config entry would be redundant.
        assert!(
            config.config_at("args").is_none(),
            "atomic list-type should be omitted (index matching is the default)"
        );
        assert!(
            config.config_at("command").is_none(),
            "unannotated array should be omitted (index matching is the default)"
        );
    }

    #[test]
    fn legacy_patch_merge_key_becomes_single_key() {
        let schema = json!({
            "properties": {
                "containers": {
                    "type": "array",
                    "x-kubernetes-patch-strategy": "merge",
                    "x-kubernetes-patch-merge-key": "name",
                    "items": {}
                }
            }
        });
        let config = match_config_from_schema(&schema, None);
        let mode = config
            .config_at("containers")
            .expect("containers configured")
            .mode();
        assert!(matches!(mode, ArrayMatchMode::Key(keys)
            if keys == &["name".to_owned()]
        ));
    }

    #[test]
    fn empty_list_map_keys_is_omitted() {
        // A `map` list-type with no map-keys would produce an empty key set,
        // which the diff algorithm rejects. Guard against emitting it.
        let schema = json!({
            "properties": {
                "weird": {
                    "type": "array",
                    "x-kubernetes-list-type": "map",
                    "items": {}
                }
            }
        });
        let config = match_config_from_schema(&schema, None);
        // Omission is intentional: emitting keys([]) would build a config the
        // diff algorithm rejects at runtime (NoDistinguishedKeys). Better to
        // omit the malformed entry than to produce a config that errors later.
        assert!(
            config.config_at("weird").is_none(),
            "map list-type without map-keys should be omitted, not emitted as an empty key set"
        );
    }

    #[test]
    fn nested_arrays_get_dotted_paths() {
        // An array nested inside an array element is recorded at the dotted
        // path, matching how the diff engine looks up config for nested arrays
        // (e.g. spec.containers.ports).
        let schema = json!({
            "properties": {
                "spec": {
                    "properties": {
                        "containers": {
                            "type": "array",
                            "x-kubernetes-list-type": "map",
                            "x-kubernetes-list-map-keys": ["name"],
                            "items": {
                                "properties": {
                                    "ports": {
                                        "type": "array",
                                        "x-kubernetes-list-type": "map",
                                        "x-kubernetes-list-map-keys": ["containerPort", "protocol"],
                                        "items": {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        let config = match_config_from_schema(&schema, None);

        let containers = config
            .config_at("spec.containers")
            .expect("spec.containers configured")
            .mode();
        assert!(matches!(containers, ArrayMatchMode::Key(keys)
            if keys == &["name".to_owned()]
        ));

        let ports = config
            .config_at("spec.containers.ports")
            .expect("spec.containers.ports configured")
            .mode();
        assert!(matches!(ports, ArrayMatchMode::Key(keys)
            if keys == &["containerPort".to_owned(), "protocol".to_owned()]
        ));
    }

    #[test]
    fn ref_is_resolved() {
        // An array whose items are a $ref must be resolved so nested arrays in
        // the referenced schema are discovered.
        let root = json!({
            "properties": {
                "spec": {
                    "properties": {
                        "containers": {
                            "type": "array",
                            "x-kubernetes-list-type": "map",
                            "x-kubernetes-list-map-keys": ["name"],
                            "items": { "$ref": "#/components/schemas/Container" }
                        }
                    }
                }
            }
        });
        let components = json!({
            "Container": {
                "properties": {
                    "ports": {
                        "type": "array",
                        "x-kubernetes-list-type": "map",
                        "x-kubernetes-list-map-keys": ["containerPort", "protocol"],
                        "items": {}
                    }
                }
            }
        });
        let config = match_config_from_schema(&root, Some(&components));

        let ports = config
            .config_at("spec.containers.ports")
            .expect("spec.containers.ports resolved via $ref")
            .mode();
        assert!(matches!(ports, ArrayMatchMode::Key(keys)
            if keys == &["containerPort".to_owned(), "protocol".to_owned()]
        ));
    }

    #[test]
    fn cyclic_ref_terminates() {
        // A self-referential schema must not recurse forever. Arrays before the
        // cycle are still recorded; the repeated ref is not descended again.
        let root = json!({ "$ref": "#/components/schemas/Node" });
        let components = json!({
            "Node": {
                "properties": {
                    "children": {
                        "type": "array",
                        "x-kubernetes-list-type": "map",
                        "x-kubernetes-list-map-keys": ["id"],
                        "items": { "$ref": "#/components/schemas/Node" }
                    }
                }
            }
        });
        let config = match_config_from_schema(&root, Some(&components));

        assert!(
            config.config_at("children").is_some(),
            "the first level of the cyclic schema should be recorded"
        );
        assert!(
            config.config_at("children.children").is_none(),
            "the cycle should stop before descending into itself again"
        );
    }

    #[test]
    fn allof_wrapped_refs_are_resolved() {
        // The real Kubernetes spec wraps every $ref in an `allOf` alongside
        // `default`/`description` metadata, both for object properties and for
        // array `items`. The walker must descend through `allOf` to resolve them.
        let root = json!({
            "properties": {
                "spec": {
                    "allOf": [{ "$ref": "#/components/schemas/PodSpec" }],
                    "default": {},
                    "description": "the pod spec"
                }
            }
        });
        let components = json!({
            "PodSpec": {
                "properties": {
                    "containers": {
                        "type": "array",
                        "x-kubernetes-list-type": "map",
                        "x-kubernetes-list-map-keys": ["name"],
                        "items": {
                            "allOf": [{ "$ref": "#/components/schemas/Container" }],
                            "default": {}
                        }
                    }
                }
            },
            "Container": {
                "properties": {
                    "ports": {
                        "type": "array",
                        "x-kubernetes-list-type": "map",
                        "x-kubernetes-list-map-keys": ["containerPort", "protocol"],
                        "items": {}
                    }
                }
            }
        });
        let config = match_config_from_schema(&root, Some(&components));

        let containers = config
            .config_at("spec.containers")
            .expect("spec.containers resolved through allOf")
            .mode();
        assert!(matches!(containers, ArrayMatchMode::Key(keys)
            if keys == &["name".to_owned()]
        ));

        let ports = config
            .config_at("spec.containers.ports")
            .expect("spec.containers.ports resolved through allOf")
            .mode();
        assert!(matches!(ports, ArrayMatchMode::Key(keys)
            if keys == &["containerPort".to_owned(), "protocol".to_owned()]
        ));
    }
}
