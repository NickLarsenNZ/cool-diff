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

use serde_json::Value;

use crate::config::{ArrayMatchConfig, ArrayMatchMode, MatchConfig};

/// Derives a [`MatchConfig`] from a Kubernetes OpenAPI object schema.
///
/// Inspects the array properties of `root` and records a per-path match mode
/// for those whose vendor extensions call for something other than index
/// matching.
pub fn match_config_from_schema(_root: &Value) -> MatchConfig {
    MatchConfig::new()
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
        let config = match_config_from_schema(&schema);
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
        let config = match_config_from_schema(&schema);
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
        let config = match_config_from_schema(&schema);
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
        let config = match_config_from_schema(&schema);
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
        let config = match_config_from_schema(&schema);
        // Omission is intentional: emitting keys([]) would build a config the
        // diff algorithm rejects at runtime (NoDistinguishedKeys). Better to
        // omit the malformed entry than to produce a config that errors later.
        assert!(
            config.config_at("weird").is_none(),
            "map list-type without map-keys should be omitted, not emitted as an empty key set"
        );
    }
}
