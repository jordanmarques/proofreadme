//! `Switch`: the two-cell OFF/ON toggle, the hardware switch of the 90s
//! (`components/forms/Switch.jsx`).

use std::sync::Arc;

use egui::{Color32, CornerRadius, Galley, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

use crate::fonts::{self, Voice};
use crate::{effects, tokens};

const CELL_HEIGHT: f32 = 24.0;
const CELL_MIN_WIDTH: f32 = 42.0;
const CELL_MARGIN: f32 = 8.0;
/// The clearance of 1 around the cells and between them.
const CLEARANCE: f32 = 1.0;
const LABEL_GAP: f32 = 12.0;
const LETTER_SPACING: f32 = 0.1;
const DISABLED_OPACITY: f32 = 0.4;

/// The Kairo toggle. A click on a cell picks it; a click on the label, or Space on the keyboard,
/// inverts the state. The response is marked `changed()` when the state has moved.
///
/// ```no_run
/// # fn screen(ui: &mut egui::Ui, automatic: &mut bool) {
/// ui.add(kairo::Switch::new(automatic).label("Auto sync").jp("自動"));
/// # }
/// ```
#[must_use = "a widget does nothing until it is added to the interface: `ui.add(...)`"]
pub struct Switch<'a> {
    state: &'a mut bool,
    label: Option<&'a str>,
    jp: Option<&'a str>,
    text_on: &'a str,
    text_off: &'a str,
    enabled: bool,
}

impl<'a> Switch<'a> {
    pub fn new(state: &'a mut bool) -> Self {
        Self { state, label: None, jp: None, text_on: "ON", text_off: "OFF", enabled: true }
    }

    /// The label to the right of the toggle, put in capitals.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// The Japanese dressing that follows the label.
    pub fn jp(mut self, jp: &'a str) -> Self {
        self.jp = Some(jp);
        self
    }

    /// The texts of the two cells, `OFF` and `ON` by default (for example `MAN` and `AUTO`).
    pub fn texts(mut self, off: &'a str, on: &'a str) -> Self {
        self.text_off = off;
        self.text_on = on;
        self
    }

    /// A disabled toggle is dimmed and does not react to a click.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let on = *self.state;
        // The two cells are coloured at paint time, once the click has been taken into account.
        let cell = |text: &str| -> Arc<Galley> {
            ui.painter().layout_job(fonts::text(text, Voice::Mono, tokens::TEXT_XS, LETTER_SPACING, Color32::PLACEHOLDER))
        };
        let text_off = cell(self.text_off);
        let text_on = cell(self.text_on);
        let cell_width = |galley: &Galley| (galley.size().x + 2.0 * CELL_MARGIN).max(CELL_MIN_WIDTH);
        let (width_off, width_on) = (cell_width(&text_off), cell_width(&text_on));
        // Border of 1, clearance of 1, OFF cell, clearance of 1, ON cell, clearance of 1, border of 1.
        let frame = vec2(
            2.0 * tokens::BORDER_HAIRLINE + 3.0 * CLEARANCE + width_off + width_on,
            2.0 * tokens::BORDER_HAIRLINE + 2.0 * CLEARANCE + CELL_HEIGHT,
        );
        let label = self.label.map(|label| ui.painter().layout_job(fonts::bilingual_label(label, self.jp)));
        let width = frame.x + label.as_ref().map_or(0.0, |label| LABEL_GAP + label.size().x);

        let enabled = self.enabled && ui.is_enabled();
        let (rect, mut response) = ui.allocate_exact_size(vec2(width, frame.y), if enabled { Sense::click() } else { Sense::hover() });
        let frame_rect = Rect::from_min_size(rect.min, frame);
        let inner = frame_rect.shrink(tokens::BORDER_HAIRLINE + CLEARANCE);
        let rect_off = Rect::from_min_size(inner.min, vec2(width_off, CELL_HEIGHT));
        let rect_on = Rect::from_min_size(pos2(rect_off.right() + CLEARANCE, inner.top()), vec2(width_on, CELL_HEIGHT));

        if response.clicked() {
            // A click on a cell picks it; elsewhere (label, keyboard, screen reader), the state is
            // inverted.
            let wanted = match response.interact_pointer_pos() {
                Some(position) if rect_on.contains(position) => true,
                Some(position) if rect_off.contains(position) => false,
                _ => !on,
            };
            if wanted != on {
                *self.state = wanted;
                response.mark_changed();
            }
        }
        let on = *self.state;
        let name = self.label.map(str::to_uppercase).unwrap_or_default();
        response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, on, &name));

        if ui.is_rect_visible(rect) {
            let mut painter = ui.painter().clone();
            if !self.enabled && ui.is_enabled() {
                painter.multiply_opacity(DISABLED_OPACITY);
            }
            painter.rect(frame_rect, CornerRadius::ZERO, tokens::BG_SUNKEN, Stroke::new(tokens::BORDER_HAIRLINE, tokens::LINE_2), StrokeKind::Inside);
            let (picked_rect, fill) = if on { (rect_on, tokens::K_ORANGE_500) } else { (rect_off, tokens::K_INK_600) };
            painter.rect_filled(picked_rect, CornerRadius::ZERO, fill);
            let center = |rect: Rect, galley: &Galley| rect.center() - galley.size() / 2.0;
            painter.galley_with_override_text_color(center(rect_off, &text_off), text_off.clone(), if on { tokens::FG_3 } else { tokens::FG_1 });
            painter.galley_with_override_text_color(center(rect_on, &text_on), text_on.clone(), if on { tokens::K_VOID } else { tokens::FG_3 });
            if let Some(label) = label {
                painter.galley(pos2(frame_rect.right() + LABEL_GAP, rect.center().y - label.size().y / 2.0), label, tokens::FG_2);
            }
            if response.has_focus() {
                effects::focus_ring(&painter, frame_rect);
            }
        }
        response
    }
}
