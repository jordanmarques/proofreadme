//! `HazardStripe`: the diagonal hazard stripe that borders critical areas
//! (`components/brand/HazardStripe.jsx`).

use egui::{CornerRadius, Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, vec2};

use super::Tone;
use crate::fonts::{self, Voice};
use crate::{effects, tokens};

/// The Kairo hazard stripe, with an optional centred label. It takes up the whole available
/// width. Kairo only offers it in orange, red and amber.
#[must_use = "a widget does nothing until it is added to the interface: `ui.add(...)`"]
pub struct HazardStripe<'a> {
    tone: Tone,
    height: f32,
    label: Option<&'a str>,
}

impl<'a> HazardStripe<'a> {
    /// The orange stripe.
    pub fn orange() -> Self {
        Self { tone: Tone::Accent, height: 16.0, label: None }
    }

    /// The red stripe, for the alert.
    pub fn red() -> Self {
        Self { tone: Tone::Alert, ..Self::orange() }
    }

    /// The amber stripe, for caution.
    pub fn amber() -> Self {
        Self { tone: Tone::Warn, ..Self::orange() }
    }

    /// The height of the stripe, 16 by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// A centred label on a black background, put in capitals.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

impl Widget for HazardStripe<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let color = self.tone.color();
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let label = self.label.map(str::to_uppercase);
        if let Some(label) = &label {
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, label));
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        effects::hazard_stripes(painter, rect, color);
        if let Some(label) = &label {
            let size = (self.height * 0.42).max(11.0);
            let galley = painter.layout_job(fonts::text(label.as_str(), Voice::LabelStrong, size, tokens::TRACKING_WIDE, color));
            // `padding: 3px 12px` around the text, on a black background.
            let plate = Rect::from_center_size(rect.center(), galley.size() + vec2(24.0, 6.0));
            painter.rect_filled(plate, CornerRadius::ZERO, tokens::K_VOID);
            painter.galley(plate.center() - galley.size() / 2.0, galley, color);
        }
        response
    }
}
