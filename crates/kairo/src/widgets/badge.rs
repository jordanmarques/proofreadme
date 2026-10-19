//! `Badge`: the small mono tag for codes, states and counts (`components/core/Badge.jsx`).

use egui::{Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

use super::Tone;
use crate::fonts::{self, Voice};
use crate::tokens;

const HEIGHT: f32 = 20.0;
const MARGIN: f32 = 7.0;
const LETTER_SPACING: f32 = 0.08;

/// The Kairo mono tag: outlined by default, solid for what is urgent. The text is put in capitals.
#[must_use = "a widget does nothing until it is added to the interface: `ui.add(...)`"]
pub struct Badge<'a> {
    text: &'a str,
    tone: Tone,
    solid: bool,
}

impl<'a> Badge<'a> {
    /// A neutral, outlined badge.
    pub fn new(text: &'a str) -> Self {
        Self { text, tone: Tone::Neutral, solid: false }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// `solid` variant: background in the colour of the tone, for what is urgent.
    pub fn solid(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }
}

impl Widget for Badge<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let color = self.tone.color();
        let ink = match (self.solid, self.tone) {
            (false, _) => color,
            (true, Tone::Alert | Tone::Unit) => tokens::K_PAPER,
            (true, _) => tokens::K_VOID,
        };
        let text = self.text.to_uppercase();
        let galley = ui.painter().layout_job(fonts::text(text.as_str(), Voice::Mono, tokens::TEXT_XS, LETTER_SPACING, ink));
        let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 2.0 * MARGIN, HEIGHT), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
        if ui.is_rect_visible(rect) {
            let fill = if self.solid { color } else { Color32::TRANSPARENT };
            ui.painter().rect(rect, CornerRadius::ZERO, fill, Stroke::new(tokens::BORDER_HAIRLINE, color), StrokeKind::Inside);
            ui.painter().galley(pos2(rect.left() + MARGIN, rect.center().y - galley.size().y / 2.0), galley, ink);
        }
        response
    }
}
