//! Entry point of ProofReadMe.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
// On Windows, a release build opens no console behind the window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match proofreadme::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ProofReadMe could not start: {error}");
            ExitCode::FAILURE
        }
    }
}
