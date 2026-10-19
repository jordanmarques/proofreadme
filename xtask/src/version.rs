//! `cargo xtask version`: reads or sets the version of the workspace, the one a release carries.

use std::fs;

/// Prints the workspace version, or with `set <version>` writes it to the root `Cargo.toml` and
/// refreshes `Cargo.lock`.
pub fn version(arguments: &[String]) -> Result<(), String> {
    let manifest_path = crate::root().join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).map_err(|e| format!("reading {}: {e}", manifest_path.display()))?;
    match arguments {
        [] => {
            println!("{}", workspace_version(&manifest)?);
            Ok(())
        }
        [set, version] if set == "set" => {
            let updated = with_workspace_version(&manifest, version)?;
            fs::write(&manifest_path, updated).map_err(|e| format!("writing {}: {e}", manifest_path.display()))?;
            // The lock file names the version of every workspace crate.
            crate::run(crate::cargo().args(["update", "--workspace", "--offline"]), "cargo update --workspace --offline")?;
            eprintln!("xtask version: {version}");
            Ok(())
        }
        _ => Err("version: expected no argument, or `set <version>`".to_owned()),
    }
}

/// The index of the `version = "…"` line of `[workspace.package]`, with the lines of the manifest.
fn version_line(manifest: &str) -> Result<(Vec<&str>, usize), String> {
    let lines: Vec<&str> = manifest.lines().collect();
    let section = lines.iter().position(|line| line.trim() == "[workspace.package]").ok_or("Cargo.toml has no [workspace.package] section")?;
    let index = lines
        .iter()
        .enumerate()
        .skip(section + 1)
        .take_while(|(_, line)| !line.trim_start().starts_with('['))
        .find(|(_, line)| line.split('=').next().is_some_and(|key| key.trim() == "version"))
        .map(|(index, _)| index)
        .ok_or("[workspace.package] has no version")?;
    Ok((lines, index))
}

/// The version declared by `[workspace.package]`.
fn workspace_version(manifest: &str) -> Result<String, String> {
    let (lines, index) = version_line(manifest)?;
    let value = lines.get(index).and_then(|line| line.split_once('=')).map(|(_, value)| value.trim().trim_matches('"'));
    value.map(str::to_owned).ok_or_else(|| "[workspace.package] has no version".to_owned())
}

/// The manifest with the version of `[workspace.package]` replaced; every other line is kept.
fn with_workspace_version(manifest: &str, version: &str) -> Result<String, String> {
    if !is_a_release_version(version) {
        return Err(format!("version: `{version}` is not a version like 1.2.3 or 1.2.3-rc.1"));
    }
    let (lines, index) = version_line(manifest)?;
    let replacement = format!("version = \"{version}\"");
    let mut updated: Vec<&str> = lines;
    if let Some(line) = updated.get_mut(index) {
        *line = &replacement;
    }
    let mut text = updated.join("\n");
    if manifest.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// True for `MAJOR.MINOR.PATCH`, optionally followed by `-` and dot-separated pre-release
/// identifiers. A version with a hyphen is published as a pre-release.
fn is_a_release_version(version: &str) -> bool {
    let (core, pre_release) = match version.split_once('-') {
        Some((core, pre_release)) => (core, Some(pre_release)),
        None => (version, None),
    };
    let numbers: Vec<&str> = core.split('.').collect();
    let is_a_number = |part: &&str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()) && (*part == "0" || !part.starts_with('0'));
    let is_an_identifier = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    numbers.len() == 3 && numbers.iter().all(is_a_number) && pre_release.is_none_or(|pre_release| pre_release.split('.').all(is_an_identifier))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "[workspace]\nmembers = [\"xtask\"]\n\n[workspace.package]\nedition = \"2024\"\nversion = \"0.1.0\"\n\n[workspace.dependencies]\nversion-thing = { version = \"9.9.9\" }\n";

    #[test]
    fn the_workspace_version_is_read_from_its_own_section() {
        assert_eq!(workspace_version(MANIFEST), Ok("0.1.0".to_owned()));
    }

    #[test]
    fn setting_the_version_changes_only_the_workspace_version_line() {
        let updated = with_workspace_version(MANIFEST, "1.2.0-rc.1").unwrap();

        assert_eq!(updated, MANIFEST.replace("version = \"0.1.0\"", "version = \"1.2.0-rc.1\""));
        assert_eq!(workspace_version(&updated), Ok("1.2.0-rc.1".to_owned()));
    }

    #[test]
    fn a_version_that_is_not_semantic_is_refused() {
        for refused in ["", "1.2", "v1.2.3", "1.2.3.4", "01.2.3", "1.2.x", "1.2.3-", "1.2.3-rc..1", "1.2.3 "] {
            assert!(with_workspace_version(MANIFEST, refused).is_err(), "{refused:?} should be refused");
        }
        for accepted in ["0.1.0", "1.2.3", "10.20.30", "1.2.3-rc.1", "1.0.0-beta"] {
            assert!(is_a_release_version(accepted), "{accepted:?} should be accepted");
        }
    }

    #[test]
    fn a_manifest_without_a_workspace_version_is_an_error() {
        assert!(workspace_version("[workspace]\nmembers = []\n").is_err());
        assert!(workspace_version("[workspace.package]\nedition = \"2024\"\n\n[package]\nversion = \"1.0.0\"\n").is_err());
    }
}
