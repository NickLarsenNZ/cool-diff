//! Development tasks for cool-diff, run via `cargo xtask <command>`.
//!
//! Commands:
//! - `generate --ref <committish>`: regenerate the Kubernetes `MatchConfig`
//!   presets from the upstream OpenAPI spec at the given committish (a tag or
//!   commit SHA). Fetches over the network, so it is a deliberate developer
//!   action and never run in CI.

use std::process::ExitCode;

use indoc::indoc;

mod generate;

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

    match generate::run(&committish) {
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

fn print_usage() {
    eprint!(indoc! {"
        usage: cargo xtask <command>

        commands:
          generate --ref <committish>   regenerate the Kubernetes presets
    "});
}
