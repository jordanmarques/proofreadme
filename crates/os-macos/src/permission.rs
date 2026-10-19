//! The Accessibility permission: the synthesis of Cmd+C and Cmd+V needs it, and it also lets the
//! listen-only event tap see the keyboard.

use std::io;
use std::process::Command;

use objc2_core_graphics::{CGPreflightPostEventAccess, CGRequestPostEventAccess};

/// The Accessibility pane of Privacy & Security, in System Settings.
const PANE: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// True if the agent has the Accessibility permission.
pub(crate) fn granted() -> bool {
    CGPreflightPostEventAccess()
}

/// Asks macOS for the permission: the agent appears in the list of the pane, and macOS shows its
/// own prompt once.
pub(crate) fn request() {
    let _ = CGRequestPostEventAccess();
}

/// Opens the Accessibility pane of System Settings.
pub fn open_permission_pane() -> io::Result<()> {
    let status = Command::new("open").arg(PANE).status()?;
    if status.success() { Ok(()) } else { Err(io::Error::other(format!("open failed: {status}"))) }
}
