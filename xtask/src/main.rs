//! Development tasks for cool-diff, run via `cargo xtask <command>`.
//!
//! Commands:
//! - `generate --ref <committish>`: regenerate the Kubernetes `MatchConfig`
//!   presets from the upstream OpenAPI spec at the given committish (a tag or
//!   commit SHA). Fetches over the network, so it is a deliberate developer
//!   action and never run in CI.

use std::error::Error;
use std::process::ExitCode;

use indoc::indoc;

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

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("generate") => run_generate(args),
        Some(other) => {
            eprintln!("xtask: unknown command `{other}`");
            print_usage();
            ExitCode::FAILURE
        }
        None => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

/// Parses `generate`'s arguments and runs it, reporting any error.
fn run_generate(args: impl Iterator<Item = String>) -> ExitCode {
    let committish = match parse_ref(args) {
        Ok(committish) => committish,
        Err(message) => {
            eprintln!("xtask: {message}");
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    match generate(&committish) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Extracts the `--ref <committish>` argument.
fn parse_ref(mut args: impl Iterator<Item = String>) -> Result<String, String> {
    match args.next().as_deref() {
        Some("--ref") => args
            .next()
            .ok_or_else(|| "`--ref` needs a value".to_owned()),
        Some(other) => Err(format!("unexpected argument `{other}`, expected `--ref`")),
        None => Err("missing `--ref <committish>`".to_owned()),
    }
}

/// Fetches every spec file at `committish` and reports what was found.
///
/// Code emission is added in a later step; for now this proves the fetch and
/// parse work against the real upstream layout.
fn generate(committish: &str) -> Result<(), Box<dyn Error>> {
    let sha = resolve_sha(committish)?;

    // Report both the committish and the resolved SHA, unless they are the same.
    if committish == sha {
        eprintln!("generating from {REPO} at {sha}");
    } else {
        eprintln!("generating from {REPO} at {committish} ({sha})");
    }

    for file in SPEC_FILES {
        let path = format!("{SPEC_DIR}/{file}");
        let spec = fetch_spec(&sha, &path)?;
        let doc: serde_json::Value = serde_json::from_str(&spec)?;
        let schema_count = doc["components"]["schemas"]
            .as_object()
            .map_or(0, serde_json::Map::len);
        eprintln!("  {file}: {schema_count} schemas");
    }
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

fn print_usage() {
    eprint!(indoc! {"
        usage: cargo xtask <command>

        commands:
          generate --ref <committish>   regenerate the Kubernetes presets
    "});
}
