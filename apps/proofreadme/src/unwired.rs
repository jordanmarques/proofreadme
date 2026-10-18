//! The OS port of the agent until the platform adapters exist: no keyboard hook feeds the agent,
//! so no proofread starts, and an operation that would still reach it is refused.

use proofreadme_core::{ClipboardContent, Keystroke, Mark, Os, OsError};

/// The cause given by every refused operation.
const CAUSE: &str = "no platform adapter on this build";

/// An OS port that refuses every operation.
#[derive(Debug, Default)]
pub struct UnwiredOs;

impl Os for UnwiredOs {
    fn read_clipboard(&mut self) -> Result<ClipboardContent, OsError> {
        Err(OsError::Clipboard(CAUSE.to_owned()))
    }

    fn write_clipboard(&mut self, _content: &ClipboardContent, _mark: Mark) -> Result<(), OsError> {
        Err(OsError::Clipboard(CAUSE.to_owned()))
    }

    fn clipboard_change_count(&mut self) -> u64 {
        0
    }

    fn synthesize(&mut self, _keystroke: Keystroke) -> Result<(), OsError> {
        Err(OsError::Keystrokes(CAUSE.to_owned()))
    }

    fn foreground_application(&mut self) -> Option<String> {
        None
    }
}
