//! Kairo for egui.
//!
//! The port of the Kairo design system (ADR 0001): its tokens as constants ([`tokens`]), its
//! embedded fonts and typographic voices ([`fonts`]), its egui theme ([`theme`]), its effects
//! ([`effects`]) and its widgets. This crate is reusable without Hikyaku: it does not depend on
//! the core, and `cargo xtask ci` checks it.
//!
//! An application calls [`install`] once at startup, then composes its screens with the widgets.
//! `cargo run -p kairo --example gallery` shows them all.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod effects;
pub mod fonts;
pub mod gallery;
pub mod theme;
pub mod tokens;
mod widgets;

pub use fonts::Voice;
pub use theme::{install, ready};
pub use widgets::{Aspect, Badge, Button, HazardStripe, Light, Meter, Panel, Readout, Size, StatusLight, Switch, TitleCard, Tone};
