//! The macOS adapters of the OS port of the agent, kept thin: no rule lives here (see
//! `CODING_STANDARDS.md`). They are not unit tested; the CI build on macOS proves them.
//!
//! - [`listen`]: the keyboard hook (a listen-only `CGEventTap`) that feeds the agent its key events
//!   and reports the Accessibility permission it needs;
//! - [`MacOs`]: the pasteboard (text and image, temporary writes marked
//!   `org.nspasteboard.TransientType`), the synthesis of Cmd+C and Cmd+V, the frontmost
//!   application;
//! - [`open_permission_pane`]: the Accessibility pane of System Settings.
//!
//! Elsewhere than on macOS the crate is empty.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_os = "macos")]
mod hook;
#[cfg(target_os = "macos")]
mod os;
#[cfg(target_os = "macos")]
mod permission;

#[cfg(target_os = "macos")]
pub use hook::listen;
#[cfg(target_os = "macos")]
pub use os::MacOs;
#[cfg(target_os = "macos")]
pub use permission::open_permission_pane;

/// The value the agent writes in the user data field of the keystrokes it synthesizes: the hook
/// recognises them by it and marks them injected, so they never feed the trigger.
#[cfg(target_os = "macos")]
const INJECTED: i64 = 0x5052_4d45; // "PRME"
