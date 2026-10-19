//! The Windows adapters of the OS port of the agent: the low-level keyboard hook, the synthesized
//! Ctrl+C and Ctrl+V, the clipboard (text and image, temporary writes excluded from the clipboard
//! history) and the foreground application. They hold no rule: the trigger, the proofread and its
//! delays live in `proofreadme-core`.
//!
//! The only crate of the workspace allowed `unsafe`, for the calls to the Win32 API, each block
//! with the reason it is sound.
#![cfg(windows)]
#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn, clippy::undocumented_unsafe_blocks, clippy::multiple_unsafe_ops_per_block)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod clipboard;
mod foreground;
mod hook;
mod keystrokes;

use proofreadme_core::{ClipboardContent, Keystroke, Mark, Os, OsError};

pub use hook::{HookError, listen};

/// The OS port of the agent on Windows.
#[derive(Debug, Default)]
pub struct WindowsOs;

impl Os for WindowsOs {
    fn read_clipboard(&mut self) -> Result<ClipboardContent, OsError> {
        clipboard::read()
    }

    fn write_clipboard(&mut self, content: &ClipboardContent, mark: Mark) -> Result<(), OsError> {
        clipboard::write(content, mark)
    }

    fn clipboard_change_count(&mut self) -> u64 {
        clipboard::sequence_number()
    }

    fn synthesize(&mut self, keystroke: Keystroke) -> Result<(), OsError> {
        keystrokes::send(keystroke)
    }

    fn foreground_application(&mut self) -> Option<String> {
        foreground::identity()
    }
}
