//! The port of the agent towards the OS: the key events it receives, the clipboard, the keystrokes
//! it synthesizes and the foreground application. The platform adapters implement it and hold no
//! rule.

use std::time::Instant;

/// A modifier key that can serve as the trigger. Left and right keys are the same modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// Ctrl (Control on macOS).
    Ctrl,
    /// Alt, Option on macOS.
    Alt,
    /// Shift.
    Shift,
    /// Cmd on macOS, the Windows key on Windows.
    Cmd,
}

/// A key of the keyboard, as far as the trigger is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A modifier key.
    Modifier(Modifier),
    /// Any other key, by the code of the platform.
    Other(u32),
}

/// Whether a key goes down or up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The key is pressed (an auto-repeat of a held key is a press too).
    Down,
    /// The key is released.
    Up,
}

/// A key event seen by the keyboard hook of the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key.
    pub key: Key,
    /// Down or up.
    pub direction: Direction,
    /// When the hook saw it.
    pub at: Instant,
    /// True when the event was synthesized (by the agent or another program) rather than typed:
    /// the trigger ignores it, so the copy and paste of the agent never feed the trigger.
    pub injected: bool,
}

/// An image of the clipboard, in RGBA, 4 bytes per pixel, row by row.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// The pixels.
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image({}x{})", self.width, self.height)
    }
}

/// What the clipboard holds, in the formats the agent saves and restores: text and image (ADR
/// 0002). Both empty is an empty clipboard.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClipboardContent {
    /// The text, if any.
    pub text: Option<String>,
    /// The image, if any.
    pub image: Option<Image>,
}

impl ClipboardContent {
    /// A clipboard holding only this text.
    pub fn text(text: String) -> Self {
        Self { text: Some(text), image: None }
    }
}

/// How a write of the clipboard is marked for clipboard histories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// A temporary write of the agent: clipboard histories ignore it
    /// (`org.nspasteboard.TransientType` on macOS, the exclusion formats on Windows).
    Transient,
    /// A write meant to stay, that clipboard histories keep.
    Kept,
}

/// A keystroke the agent synthesizes in the foreground application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keystroke {
    /// Ctrl+C, Cmd+C on macOS.
    Copy,
    /// Ctrl+V, Cmd+V on macOS.
    Paste,
}

/// An operation the OS refused. Its detail never carries the text of the user.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OsError {
    /// The clipboard could not be read or written.
    #[error("clipboard unavailable: {0}")]
    Clipboard(String),
    /// The keystrokes could not be synthesized.
    #[error("keystrokes refused: {0}")]
    Keystrokes(String),
}

/// The OS as the agent sees it. Called only from the thread of a proofread, one at a time.
pub trait Os: Send + 'static {
    /// Reads the text and the image of the clipboard.
    fn read_clipboard(&mut self) -> Result<ClipboardContent, OsError>;

    /// Replaces the clipboard by this content (an empty content clears it), with this mark.
    fn write_clipboard(&mut self, content: &ClipboardContent, mark: Mark) -> Result<(), OsError>;

    /// A counter that changes each time the clipboard is written by anyone (sequence number on
    /// Windows, `changeCount` on macOS): the agent watches it to know that the copy happened.
    fn clipboard_change_count(&mut self) -> u64;

    /// Synthesizes the keystroke in the foreground application, marked as injected.
    fn synthesize(&mut self, keystroke: Keystroke) -> Result<(), OsError>;

    /// The identity of the foreground application (process, window), if the OS gives one.
    fn foreground_application(&mut self) -> Option<String>;
}
