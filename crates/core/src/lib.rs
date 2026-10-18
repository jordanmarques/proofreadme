//! The core of ProofReadMe.
//!
//! Holds every rule of the agent: the trigger, the proofread, the cleaning of the correction and
//! the settings (see `GLOSSARY.md`). This crate depends neither on egui, nor on eframe, nor on
//! anything graphical: `cargo xtask ci` checks it.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
