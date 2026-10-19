//! The Kairo widgets, redrawn for egui after `components/**/*.jsx` of the design system.

mod badge;
mod button;
mod hazard_stripe;
mod meter;
mod panel;
mod readout;
mod status_light;
mod switch;
mod title_card;

use std::sync::Arc;

use egui::{Color32, Galley, Painter, Pos2, pos2};

pub use badge::Badge;
pub use button::Button;
pub use hazard_stripe::HazardStripe;
pub use meter::Meter;
pub use panel::Panel;
pub use readout::Readout;
pub use status_light::{Light, StatusLight};
pub use switch::Switch;
pub use title_card::TitleCard;

use crate::tokens;

/// The tone of a widget: the colour that carries its meaning. Orange commands, red alerts, amber
/// warns; green, cyan and violet are reserved for data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tone {
    /// `accent`: orange, the command colour.
    #[default]
    Accent,
    /// `alert`: red, reserved for the alert.
    Alert,
    /// `warn`: amber, caution.
    Warn,
    /// `ok`: phosphor green.
    Ok,
    /// `info`: cyan.
    Info,
    /// `unit`: violet.
    Unit,
    /// `neutral`: grey ink.
    Neutral,
}

impl Tone {
    /// The colour of the tone.
    pub fn color(self) -> Color32 {
        match self {
            Self::Accent => tokens::K_ORANGE_500,
            Self::Alert => tokens::K_RED_500,
            Self::Warn => tokens::K_AMBER_500,
            Self::Ok => tokens::K_PHOSPHOR_500,
            Self::Info => tokens::K_CYAN_500,
            Self::Unit => tokens::K_VIOLET_500,
            Self::Neutral => tokens::K_INK_400,
        }
    }
}

/// The three Kairo sizes: `sm`, `md`, `lg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}

/// The aspect of an interactive widget. Used to freeze a state to show it without a pointer
/// (gallery, captures): see [`Button::frozen`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aspect {
    Rest,
    Hover,
    Pressed,
}

/// The baseline of the first line of a laid-out text, measured from its top.
fn baseline(galley: &Galley) -> f32 {
    galley.rows.first().map_or(0.0, |row| row.pos.y + row.row.glyphs.first().map_or(row.row.size.y, |glyph| glyph.pos.y))
}

/// Paints a text, its baseline resting at the given height. Returns the abscissa of its end.
fn paint_on_line(painter: &Painter, left: f32, base: f32, galley: &Arc<Galley>) -> f32 {
    painter.galley(position_on_line(left, base, galley), Arc::clone(galley), Color32::PLACEHOLDER);
    left + galley.size().x
}

/// The top left corner of a text whose baseline rests at the given height.
fn position_on_line(left: f32, base: f32, galley: &Galley) -> Pos2 {
    pos2(left, base - baseline(galley))
}
