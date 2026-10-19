//! The core of ProofReadMe.
//!
//! Holds every rule of the agent: the trigger, the proofread, the cleaning of the correction and
//! the settings (see `GLOSSARY.md`). This crate depends neither on egui, nor on eframe, nor on
//! anything graphical: `cargo xtask ci` checks it.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod agent;
mod cleaning;
mod error;
mod log;
mod notification;
mod os;
mod proofread;
mod provider;
mod settings;
mod storage;
mod trigger;
mod wait;

pub use agent::{Activity, Agent, IconState, ProviderTest, State, Step, TEST_SENTENCE, Timing};
pub use error::ProofreadError;
pub use notification::{Notification, Notifier};
pub use os::{ClipboardContent, Direction, Image, Key, KeyEvent, Keystroke, Mark, Modifier, Os, OsError};
pub use provider::{PendingCorrection, request_correction};
pub use settings::{
    DEFAULT_BASE_URL, DEFAULT_MODEL, DEFAULT_PROMPT, DEFAULT_TIMEOUT, DEFAULT_TRIGGER, PLACEHOLDER, Provider, Settings, SettingsError,
};
pub use storage::{Keychain, KeychainError, MemoryKeychain, Storage};
pub use trigger::DOUBLE_PRESS_WINDOW;
