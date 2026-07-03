//! Fetches Kubernetes OpenAPI schemas and emits `MatchConfig` presets.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write as _;
use std::process::Command;

use cool_diff::{ArrayMatchConfig, ArrayMatchMode, MatchConfig, k8s_schema};

/// The upstream repository the OpenAPI spec is fetched from.
const REPO: &str = "kubernetes/kubernetes";

/// Directory holding the per-group-version OpenAPI v3 files.
const SPEC_DIR: &str = "api/openapi-spec/v3";

/// The group-version spec files to generate from. Each is a self-contained v3
/// file (its `components/schemas` bundles every type it references). Cover more
/// kinds by adding files here, e.g. `apis__apps__v1_openapi.json`.
///
// TODO: list `{SPEC_DIR}` via the GitHub contents API and warn about any v3
// files not in this list, so newly-added group-versions upstream don't get
// silently missed on regeneration.
const SPEC_FILES: &[&str] = &["api__v1_openapi.json"];

/// The kinds to generate, as `(group, version, kind)`. The group is empty for
/// the core group. Extend this to cover more resources.
const ALLOWLIST: &[(&str, &str, &str)] = &[
    ("", "v1", "ConfigMap"),
    ("", "v1", "Pod"),
    ("", "v1", "Service"),
];

/// Where the generated module is written, relative to the xtask crate.
const OUT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/kubernetes.rs");

/// A generated preset: its source schema name and derived config.
struct GeneratedKind {
    /// The OpenAPI schema name, e.g. `io.k8s.api.core.v1.Pod`.
    source: String,

    /// The config derived from that schema.
    config: MatchConfig,
}

/// Generated kinds, grouped `group_module -> version -> fn_name -> kind`.
/// `BTreeMap`s keep the output deterministic.
type Tree = BTreeMap<String, BTreeMap<String, BTreeMap<String, GeneratedKind>>>;

/// Fetches the spec at `committish`, derives configs for the allowlisted kinds,
/// and writes the generated module.
pub fn run(committish: &str) -> Result<(), Box<dyn Error>> {
    let sha = resolve_sha(committish)?;
    if committish == sha {
        eprintln!("generating from {REPO} at {sha}");
    } else {
        eprintln!("generating from {REPO} at {committish} ({sha})");
    }

    let mut tree: Tree = BTreeMap::new();
    let mut count = 0usize;

    for file in SPEC_FILES {
        let path = format!("{SPEC_DIR}/{file}");
        let spec = fetch_spec(&sha, &path)?;
        let doc: serde_json::Value = serde_json::from_str(&spec)?;
        let components = &doc["components"]["schemas"];
        let Some(schemas) = components.as_object() else {
            continue;
        };

        for (name, schema) in schemas {
            let Some((group, version, kind)) = gvk(schema) else {
                continue;
            };
            if !ALLOWLIST.contains(&(group, version, kind)) {
                continue;
            }

            let config = k8s_schema::match_config_from_schema(schema, Some(components));
            tree.entry(group_module(group))
                .or_default()
                .entry(version.to_owned())
                .or_default()
                .insert(
                    snake_case(kind),
                    GeneratedKind {
                        source: name.clone(),
                        config,
                    },
                );
            count += 1;
        }
    }

    if tree.is_empty() {
        return Err("no kinds matched the allowlist".into());
    }

    std::fs::write(OUT_PATH, render(committish, &sha, &tree))?;
    rustfmt(OUT_PATH);
    eprintln!("wrote {OUT_PATH} ({count} kinds)");
    Ok(())
}

/// Resolves a committish (tag or SHA) to a commit SHA via the GitHub API.
fn resolve_sha(committish: &str) -> Result<String, Box<dyn Error>> {
    let url = format!("https://api.github.com/repos/{REPO}/commits/{committish}");
    let body = ureq::get(&url)
        .set("User-Agent", "cool-diff-xtask")
        .call()?
        .into_string()?;
    let commit: serde_json::Value = serde_json::from_str(&body)?;
    commit["sha"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("no `sha` in GitHub response for `{committish}`").into())
}

/// Fetches a file from the repository at an immutable commit SHA.
fn fetch_spec(sha: &str, path: &str) -> Result<String, Box<dyn Error>> {
    let url = format!("https://raw.githubusercontent.com/{REPO}/{sha}/{path}");
    Ok(ureq::get(&url)
        .set("User-Agent", "cool-diff-xtask")
        .call()?
        .into_string()?)
}

/// Reads the first `x-kubernetes-group-version-kind` entry as `(group, version, kind)`.
///
/// Returns `None` for schemas without a GVK (embedded types, not resources).
fn gvk(schema: &serde_json::Value) -> Option<(&str, &str, &str)> {
    let entry = schema
        .get("x-kubernetes-group-version-kind")?
        .as_array()?
        .first()?;
    let group = entry.get("group")?.as_str()?;
    let version = entry.get("version")?.as_str()?;
    let kind = entry.get("kind")?.as_str()?;
    Some((group, version, kind))
}

/// Maps an API group to a module name: the core (empty) group becomes `core`,
/// and dots become underscores (e.g. `events.k8s.io` -> `events_k8s_io`).
fn group_module(group: &str) -> String {
    if group.is_empty() {
        "core".to_owned()
    } else {
        group.replace('.', "_")
    }
}

/// Converts a PascalCase kind to a snake_case function name, keeping runs of
/// capitals together (e.g. `ConfigMap` -> `config_map`, `CSINode` -> `csi_node`).
fn snake_case(kind: &str) -> String {
    let chars: Vec<char> = kind.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_lower = i > 0 && chars[i - 1].is_lowercase();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            if i > 0 && (prev_lower || next_lower) {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Renders the generated module source (unformatted; `rustfmt` tidies it).
fn render(committish: &str, sha: &str, tree: &Tree) -> String {
    let mut source = String::new();
    let _ = writeln!(
        source,
        "//! Pre-generated `MatchConfig`s for well-known Kubernetes types."
    );
    let _ = writeln!(source, "//!");
    let _ = writeln!(
        source,
        "//! GENERATED by `cargo xtask generate`. Do not edit by hand."
    );
    let _ = writeln!(source, "//!");
    if committish == sha {
        let _ = writeln!(source, "//! Source: {REPO} @ `{sha}`.");
    } else {
        let _ = writeln!(source, "//! Source: {REPO} @ `{committish}` (`{sha}`).");
    }
    let _ = writeln!(source);

    for (group, versions) in tree {
        let _ = writeln!(source, "/// The `{group}` API group.");
        let _ = writeln!(source, "pub mod {group} {{");
        for (version, kinds) in versions {
            // Only import the builder types when a kind in this module uses them.
            //
            // TODO: `crate::` only resolves inside cool-diff. In future, to let
            // downstream build.rs generate CRD presets, the emitted path must be
            // parameterised (`crate` internally, `cool_diff` externally) and the
            // emitter promoted from the xtask into the library.
            let has_entries = kinds.values().any(|k| k.config.paths().next().is_some());
            let _ = writeln!(source, "/// The `{version}` API version.");
            let _ = writeln!(source, "pub mod {version} {{");
            if has_entries {
                let _ = writeln!(
                    source,
                    "use crate::{{ArrayMatchConfig, ArrayMatchMode, MatchConfig}};"
                );
            } else {
                let _ = writeln!(source, "use crate::MatchConfig;");
            }

            for (fn_name, kind) in kinds {
                let _ = writeln!(source, "/// `MatchConfig` for `{}`.", kind.source);
                let _ = writeln!(source, "pub fn {fn_name}() -> MatchConfig {{");
                let _ = write!(source, "MatchConfig::new()");

                let mut entries: Vec<(&str, &ArrayMatchConfig)> = kind.config.paths().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                for (path, config) in entries {
                    let _ = write!(
                        source,
                        ".with_config_at({path:?}, ArrayMatchConfig::new({}))",
                        render_mode(config.mode()),
                    );
                }
                let _ = writeln!(source, "\n}}");
            }
            let _ = writeln!(source, "}}");
        }
        let _ = writeln!(source, "}}");
    }
    source
}

/// Renders an [`ArrayMatchMode`] as the builder call that produces it.
fn render_mode(mode: &ArrayMatchMode) -> String {
    match mode {
        ArrayMatchMode::Index => "ArrayMatchMode::Index".to_owned(),
        ArrayMatchMode::Contains => "ArrayMatchMode::Contains".to_owned(),
        ArrayMatchMode::Key(keys) if keys.len() == 1 => {
            format!("ArrayMatchMode::key({:?})", keys[0])
        }
        ArrayMatchMode::Key(keys) => {
            let list: Vec<String> = keys.iter().map(|k| format!("{k:?}")).collect();
            format!("ArrayMatchMode::keys([{}])", list.join(", "))
        }
    }
}

/// Formats the generated file in place, warning (not failing) if `rustfmt` is
/// unavailable.
fn rustfmt(path: &str) {
    match Command::new("rustfmt")
        .args(["--edition", "2024", path])
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(_) => eprintln!("warning: rustfmt failed on {path}"),
        Err(error) => eprintln!("warning: could not run rustfmt: {error}"),
    }
}
