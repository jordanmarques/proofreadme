//! Build script: on Windows, embeds in `proofreadme.exe` its icon and its version information, for
//! Explorer, the taskbar and the Task Manager.
//!
//! The icon is produced here by the `proofreadme-icon` crate (no image file in the repository) and
//! written to the build output folder, on every OS; only the embedding is specific to Windows. It
//! goes through the `winresource` crate, which calls the resource compiler of the Windows SDK
//! (`rc.exe`, installed with the MSVC build tools).
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    println!("cargo:rerun-if-changed=build.rs");
    match resources() {
        Ok(()) => ExitCode::SUCCESS,
        // A development build does without an icon; a release build without an icon would not be
        // the expected package.
        Err(error) if std::env::var("PROFILE").as_deref() == Ok("release") => {
            eprintln!("resources of the executable: {error}");
            ExitCode::FAILURE
        }
        Err(error) => {
            println!("cargo:warning=proofreadme is built without an icon or version information: {error}");
            ExitCode::SUCCESS
        }
    }
}

fn resources() -> Result<(), String> {
    let output = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let icon = output.join("proofreadme.ico");
    std::fs::write(&icon, proofreadme_icon::ico()).map_err(|error| format!("writing {}: {error}", icon.display()))?;
    embed(&icon)
}

#[cfg(windows)]
fn embed(icon: &Path) -> Result<(), String> {
    let icon = icon.to_str().ok_or("the path of the icon is not UTF-8")?;
    winresource::WindowsResource::new()
        .set_icon(icon)
        .set("ProductName", "ProofReadMe")
        .set("FileDescription", "ProofReadMe")
        .set("OriginalFilename", "ProofReadMe.exe")
        .set("InternalName", "proofreadme")
        .set("CompanyName", "Jordan Marques")
        .set("LegalCopyright", "Copyright © Jordan Marques. MIT OR Apache-2.0.")
        .compile()
        .map_err(|error| error.to_string())
}

/// Outside Windows, the executable carries no resources.
#[cfg(not(windows))]
fn embed(_icon: &Path) -> Result<(), String> {
    Ok(())
}
