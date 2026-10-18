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
mod os;
mod proofread;
mod provider;
mod settings;
mod trigger;
mod wait;

pub use agent::{Activity, Agent, State, Step, Timing};
pub use error::ProofreadError;
pub use os::{ClipboardContent, Direction, Image, Key, KeyEvent, Keystroke, Mark, Modifier, Os, OsError};
pub use provider::{PendingCorrection, request_correction};
pub use settings::{PLACEHOLDER, Provider, Settings};
pub use trigger::DOUBLE_PRESS_WINDOW;
