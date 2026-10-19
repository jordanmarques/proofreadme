//! `Meter`: the segmented gauge (`components/data/Meter.jsx`).

use egui::emath::GuiRounding as _;
use egui::{Color32, CornerRadius, Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

use super::{Tone, paint_on_line};
use crate::fonts;
use crate::tokens;

const LINE_GAP: f32 = 6.0;
const SEGMENT_GAP: f32 = 2.0;

/// The Kairo segmented gauge. With no imposed tone, it goes from green to amber then to red as it
/// fills up. It takes up the whole available width.
///
/// ```no_run
/// # fn screen(ui: &mut egui::Ui) {
/// ui.add(kairo::Meter::new(67.0, 100.0).label("Progress").jp("進捗"));
/// # }
/// ```
#[must_use = "a widget does nothing until it is added to the interface: `ui.add(...)`"]
pub struct Meter<'a> {
    value: f32,
    max: f32,
    label: Option<&'a str>,
    jp: Option<&'a str>,
    segments: usize,
    tone: Option<Tone>,
    show_value: bool,
    readout: Option<&'a str>,
    height: f32,
}

impl<'a> Meter<'a> {
    /// A gauge filled to `value / max`, bounded between empty and full.
    pub fn new(value: f32, max: f32) -> Self {
        Self { value, max, label: None, jp: None, segments: 24, tone: None, show_value: true, readout: None, height: 12.0 }
    }

    /// The label above the gauge, put in capitals.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// The Japanese dressing that follows the label.
    pub fn jp(mut self, jp: &'a str) -> Self {
        self.jp = Some(jp);
        self
    }

    /// The number of segments, 24 by default.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }

    /// Imposes one colour on all the lit segments, instead of green → amber → red.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Shows or hides the percentage to the right of the label (shown by default).
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Replaces the percentage with a readout of your choice, to the right of the label
    /// (`128 / 412`). It joins the accessible name of the gauge, after the label.
    pub fn readout(mut self, readout: &'a str) -> Self {
        self.readout = Some(readout);
        self
    }

    /// The height of the segments, 12 by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

/// The filled share, between 0 and 1. A gauge with no valid maximum is empty.
fn share(value: f32, max: f32) -> f32 {
    let share = value / max;
    if share.is_finite() { share.clamp(0.0, 1.0) } else { 0.0 }
}

/// The number of lit segments for a filled share.
fn lit_segments(share: f32, segments: usize) -> usize {
    ((share * segments as f32).round() as usize).min(segments)
}

/// The colour of segment `index` in automatic tone: red from 85% of the gauge, amber from 65%,
/// green before.
fn automatic_color(index: usize, segments: usize) -> Color32 {
    let rank = index as f32 / segments as f32;
    if rank >= 0.85 {
        tokens::K_RED_500
    } else if rank >= 0.65 {
        tokens::K_AMBER_500
    } else {
        tokens::K_PHOSPHOR_500
    }
}

impl Widget for Meter<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let share = share(self.value, self.max);
        let percentage = format!("{:.1}%", share * 100.0);
        let painter = ui.painter();
        let label = self.label.map(|label| painter.layout_job(fonts::bilingual_label(label, self.jp)));
        let readout = self.readout.or(self.show_value.then_some(percentage.as_str()));
        let value = readout.map(|readout| painter.layout_job(fonts::mono(readout, 12.0, tokens::FG_1)));
        let line_height = label.iter().chain(value.iter()).map(|galley| galley.size().y).fold(0.0, f32::max);
        let text_height = if line_height > 0.0 { line_height + LINE_GAP } else { 0.0 };

        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), text_height + self.height), Sense::hover());
        let name = format!("{} {}", self.label.map(str::to_uppercase).unwrap_or_default(), self.readout.unwrap_or_default());
        let name = name.trim();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ProgressIndicator, true, name);
            info.value = Some(f64::from(share));
            info
        });
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        // Label and value share their baseline, set by the taller of the two.
        let base = rect.top() + label.iter().chain(value.iter()).map(|galley| super::baseline(galley)).fold(0.0, f32::max);
        if let Some(label) = &label {
            paint_on_line(painter, rect.left(), base, label);
        }
        if let Some(value) = &value {
            paint_on_line(painter, rect.right() - value.size().x, base, value);
        }

        let segments = self.segments.max(1);
        let lit = lit_segments(share, segments);
        let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - self.height), rect.max);
        let step = (bar.width() + SEGMENT_GAP) / segments as f32;
        let density = ui.pixels_per_point();
        for index in 0..segments {
            let left = bar.left() + step * index as f32;
            let segment = Rect::from_min_max(pos2(left, bar.top()), pos2(left + step - SEGMENT_GAP, bar.bottom())).round_to_pixels(density);
            let color = match (index < lit, self.tone) {
                (false, _) => tokens::K_INK_800,
                (true, Some(tone)) => tone.color(),
                (true, None) => automatic_color(index, segments),
            };
            painter.rect_filled(segment, CornerRadius::ZERO, color);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::{automatic_color, lit_segments, share};
    use crate::tokens;

    #[test]
    fn two_thirds_of_a_gauge_of_24_light_16_segments() {
        assert_eq!(lit_segments(share(67.0, 100.0), 24), 16);
    }

    #[test]
    fn the_gauge_is_bounded_between_empty_and_full() {
        assert_eq!(lit_segments(share(-5.0, 100.0), 24), 0);
        assert_eq!(lit_segments(share(250.0, 100.0), 24), 24);
    }

    #[test]
    fn a_gauge_with_no_maximum_is_empty() {
        assert_eq!(lit_segments(share(3.0, 0.0), 24), 0);
        assert_eq!(lit_segments(share(0.0, 0.0), 24), 0);
    }

    #[test]
    fn the_automatic_tone_turns_amber_then_red() {
        // Out of 20 segments: 65% falls on segment 13, 85% on segment 17.
        assert_eq!(automatic_color(12, 20), tokens::K_PHOSPHOR_500);
        assert_eq!(automatic_color(13, 20), tokens::K_AMBER_500);
        assert_eq!(automatic_color(16, 20), tokens::K_AMBER_500);
        assert_eq!(automatic_color(17, 20), tokens::K_RED_500);
    }
}
