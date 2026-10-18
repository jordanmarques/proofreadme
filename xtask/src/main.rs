//! Tooling of the repository: `cargo xtask <command>`.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::PathBuf;
use std::process::{Command, ExitCode};

const HELP: &str = "\
cargo xtask <command>

  ci       format, clippy without a warning, layers, tests (stops at the first failure)
";

/// The layers of the workspace: each crate and the crates it is not allowed to pull in.
const LAYERS: &[(&str, &[&str])] =
    &[("proofreadme-core", &["egui", "eframe", "kairo", "winit", "keyring", "dirs"]), ("kairo", &["proofreadme-core", "proofreadme"])];

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    let result = match command.as_deref() {
        Some("ci") => ci(),
        Some("help" | "--help" | "-h") | None => {
            print!("{HELP}");
            Ok(())
        }
        Some(unknown) => Err(format!("unknown command: {unknown}\n\n{HELP}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The single check of the repository, run by hand and by the CI.
fn ci() -> Result<(), String> {
    run(cargo().args(["fmt", "--all", "--", "--check"]), "cargo fmt --all -- --check")?;
    run(
        cargo().args(["clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"]),
        "cargo clippy --workspace --all-targets --locked -- -D warnings",
    )?;
    layers()?;
    run(cargo().args(["test", "--workspace", "--locked"]), "cargo test --workspace --locked")?;
    eprintln!("xtask ci: all green");
    Ok(())
}

/// Checks that the dependency only goes downwards: the core without egui, kairo without the core.
fn layers() -> Result<(), String> {
    for (checked_crate, forbidden) in LAYERS {
        let description = format!("cargo tree -p {checked_crate} (layers)");
        eprintln!("$ {description}");
        let output = cargo()
            .args(["tree", "--locked", "-p", checked_crate, "--edges", "no-dev", "--target", "all", "--prefix", "none"])
            .output()
            .map_err(|e| format!("{description}: cannot run: {e}"))?;
        if !output.status.success() {
            return Err(format!("{description}: {}\n{}", output.status, String::from_utf8_lossy(&output.stderr)));
        }
        let found = forbidden_dependencies(&String::from_utf8_lossy(&output.stdout), forbidden);
        if !found.is_empty() {
            return Err(format!("layers: {checked_crate} must not depend on {}", found.join(", ")));
        }
    }
    Ok(())
}

/// The forbidden crates present in the output of `cargo tree --prefix none`, each named once.
fn forbidden_dependencies(tree: &str, forbidden: &[&str]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for name in tree.lines().filter_map(|line| line.split_whitespace().next()) {
        if forbidden.contains(&name) && !found.iter().any(|f| f == name) {
            found.push(name.to_owned());
        }
    }
    found
}

fn cargo() -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.current_dir(root());
    command
}

/// The root of the workspace: the parent folder of `xtask`.
fn root() -> PathBuf {
    let xtask = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xtask.parent().map(PathBuf::from).unwrap_or(xtask)
}

fn run(command: &mut Command, description: &str) -> Result<(), String> {
    eprintln!("$ {description}");
    let status = command.status().map_err(|e| format!("{description}: cannot run: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("{description}: {status}")) }
}

#[cfg(test)]
mod tests {
    use super::forbidden_dependencies;

    #[test]
    fn a_core_with_no_graphical_dependency_is_accepted() {
        let tree = "proofreadme-core v0.1.0 (/repo/crates/core)\nthiserror v2.0.12\n";

        assert!(forbidden_dependencies(tree, &["egui", "eframe"]).is_empty());
    }

    #[test]
    fn a_core_that_pulls_in_egui_is_refused_even_indirectly() {
        let tree = "proofreadme-core v0.1.0 (/repo/crates/core)\nother v1.0.0\negui v0.36.2\nemath v0.36.2\n";

        assert_eq!(forbidden_dependencies(tree, &["egui", "eframe"]), ["egui"]);
    }

    #[test]
    fn a_crate_with_a_similar_name_is_not_confused() {
        let tree = "kairo v0.1.0 (/repo/crates/kairo)\negui v0.36.2\nproofreadme-core-macros v1.0.0\n";

        assert!(forbidden_dependencies(tree, &["proofreadme-core", "proofreadme"]).is_empty());
    }

    #[test]
    fn a_kairo_that_depends_on_the_core_is_refused_only_once() {
        let tree =
            "kairo v0.1.0 (/repo/crates/kairo)\nproofreadme-core v0.1.0 (/repo/crates/core)\nproofreadme-core v0.1.0 (/repo/crates/core) (*)\n";

        assert_eq!(forbidden_dependencies(tree, &["proofreadme-core", "proofreadme"]), ["proofreadme-core"]);
    }
}
