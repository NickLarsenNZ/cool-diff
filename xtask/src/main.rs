//! Development tasks for cool-diff, run via `cargo xtask <command>`.
//!
//! Commands:
//! - `generate --ref <committish>`: regenerate the Kubernetes `MatchConfig`
//!   presets from the upstream OpenAPI spec at the given committish.

use std::process::ExitCode;

use indoc::indoc;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("generate") => {
            eprintln!("xtask: `generate` is not implemented yet");
            ExitCode::FAILURE
        }
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

fn print_usage() {
    eprint!(indoc! {"
        usage: cargo xtask <command>

        commands:
          generate --ref <committish>   regenerate the Kubernetes presets
    "});
}
