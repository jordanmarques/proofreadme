//! Tooling of the repository: `cargo xtask <command>`.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod package;
mod version;

use std::path::PathBuf;
use std::process::{Command, ExitCode};

const HELP: &str = "\
cargo xtask <command>

  ci       format, clippy without a warning, layers, tests (stops at the first failure)
  package  release build of the package of the host OS in dist/: ProofReadMe.app (macOS), signed
           with the certificate named by PROOFREADME_SIGNING_IDENTITY (default \"ProofReadMe
           Signing\"); ProofReadMe.exe (Windows)
           --ad-hoc signs the macOS bundle ad hoc instead of with the certificate
           --universal builds the macOS bundle for Apple silicon and Intel
           --dmg also writes dist/ProofReadMe.dmg (macOS)
           --msi also writes dist/ProofReadMe.msi, the installer (Windows, needs the `wix` tool)
  version  prints the workspace version; `version set 1.2.0` writes it (Cargo.toml and Cargo.lock)
";

/// The layers of the workspace: each crate and the crates it is not allowed to pull in.
const LAYERS: &[(&str, &[&str])] = &[
    (
        "proofreadme-core",
        &[
            "egui",
            "eframe",
            "kairo",
            "winit",
            "keyring",
            "dirs",
            "tray-icon",
            "auto-launch",
            "interprocess",
            "proofreadme-os-windows",
            "proofreadme-os-macos",
            "arboard",
            "clipboard-win",
        ],
    ),
    // The platform adapters stay thin: no window, no theme, no application.
    ("proofreadme-os-macos", &["egui", "eframe", "kairo", "proofreadme"]),
    ("kairo", &["proofreadme-core", "proofreadme"]),
];

/// The attribute that lifts the denial of `unsafe` (spelt in two parts so that this file does not
/// contain it).
const ALLOW_UNSAFE: &str = concat!("allow(", "unsafe_code)");

/// The only folders whose code may lift `unsafe_code`: the platform adapters.
const UNSAFE_ALLOWED: &[&str] = &["crates/os-windows"];

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.first().map(String::as_str) {
        Some("ci") => ci(),
        Some("package") => package::package(&arguments[1..]),
        Some("version") => version::version(&arguments[1..]),
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
    unsafe_confined()?;
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

/// Checks that `unsafe_code` is lifted nowhere outside [`UNSAFE_ALLOWED`].
fn unsafe_confined() -> Result<(), String> {
    eprintln!("$ unsafe confined to {}", UNSAFE_ALLOWED.join(", "));
    let root = root();
    let mut sources = Vec::new();
    for folder in ["crates", "apps", "xtask"] {
        collect_sources(&root.join(folder), &mut sources)?;
    }
    let mut offenders = Vec::new();
    for path in sources {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let relative = path.strip_prefix(&root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        if lifts_unsafe(&relative, &text, UNSAFE_ALLOWED) {
            offenders.push(relative);
        }
    }
    if offenders.is_empty() { Ok(()) } else { Err(format!("unsafe code allowed outside {}: {}", UNSAFE_ALLOWED.join(", "), offenders.join(", "))) }
}

/// The `.rs` files under `folder`, `target` folders left out.
fn collect_sources(folder: &std::path::Path, sources: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", folder.display()))?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "target") {
                collect_sources(&path, sources)?;
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    Ok(())
}

/// True if this source, at this path relative to the root, lifts `unsafe_code` outside the allowed
/// folders.
fn lifts_unsafe(path: &str, text: &str, allowed: &[&str]) -> bool {
    let inside = allowed.iter().any(|folder| path.starts_with(&format!("{folder}/")));
    !inside && text.lines().any(|line| line.contains(ALLOW_UNSAFE) && !line.trim_start().starts_with("//"))
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
    use super::{ALLOW_UNSAFE, forbidden_dependencies, lifts_unsafe};

    const ALLOWED: &[&str] = &["crates/os-windows"];

    #[test]
    fn unsafe_code_is_allowed_in_the_windows_adapters() {
        assert!(!lifts_unsafe("crates/os-windows/src/lib.rs", &format!("#![{ALLOW_UNSAFE}]\n"), ALLOWED));
    }

    #[test]
    fn unsafe_code_allowed_anywhere_else_is_refused() {
        assert!(lifts_unsafe("apps/proofreadme/src/lib.rs", &format!("#[{ALLOW_UNSAFE}]\nmod hook;\n"), ALLOWED));
        assert!(lifts_unsafe("crates/os-windows-extra/src/lib.rs", &format!("#![{ALLOW_UNSAFE}]\n"), ALLOWED));
    }

    #[test]
    fn a_crate_forbidding_unsafe_code_or_a_comment_about_it_is_accepted() {
        assert!(!lifts_unsafe("crates/core/src/lib.rs", &format!("#![forbid(unsafe_code)]\n// no {ALLOW_UNSAFE} here\n"), ALLOWED));
    }

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
