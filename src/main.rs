//! The `bloat` command-line entrypoint.
//!
//! See `docs/RUST_GUIDELINES.md` for the coding rules enforced in this
//! crate.

use std::process::ExitCode;

fn main() -> ExitCode {
    match bloatdroid::tui::run() {
        Ok(()) => {
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("bloat: {error}");
            return ExitCode::FAILURE;
        }
    }
}
