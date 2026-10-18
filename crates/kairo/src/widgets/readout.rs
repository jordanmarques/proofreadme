//! `Readout`: the large phosphor numeric readout (`components/data/Readout.jsx`).

use egui::{Response, Sense, Ui, Widget, WidgetInfo, WidgetType, vec2};

use super::{Size, Tone, baseline, paint_on_line, position_on_line};
use crate::tokens::{self, Glow};
use crate::{effects, fonts};

const LINE_GAP: f32 = 6.0;
const UNIT_GAP: f32 = 6.0;

/// The Kairo numeric readout: label, large glowing mono value, unit, detail line.
///
/// ```no_run
/// # fn screen(ui: &mut egui::Ui) {
/// ui.add(kairo::Readout::new("128").label("Copied").jp("複写").unit("/ 412"));
/// # }
/// ```
#[must_use = "a widget does nothing until it is added to the interface: `ui.add(...)`"]
pub struct Readout<'a> {
    value: &'a str,
    label: Option<&'a str>,
    jp: Option<&'a str>,
    unit: Option<&'a str>,
    detail: Option<&'a str>,
    tone: Tone,
    size: Size,
    align_right: bool,
    glow: bool,
}

impl<'a> Readout<'a> {
    /// A green readout, of medium size (40), with its glow.
    pub fn new(value: &'a str) -> Self {
        Self { value, label: None, jp: None, unit: None, detail: None, tone: Tone::Ok, size: Size::Md, align_right: false, glow: true }
    }

    /// The label above the value, put in capitals.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// The Japanese dressing that follows the label.
    pub fn jp(mut self, jp: &'a str) -> Self {
        self.jp = Some(jp);
        self
    }

    /// The unit, to the right of the value.
    pub fn unit(mut self, unit: &'a str) -> Self {
        self.unit = Some(unit);
        self
    }

    /// The detail line under the value (`delta` in Kairo).
    pub fn detail(mut self, detail: &'a str) -> Self {
        self.detail = Some(detail);
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// The size of the value: 24, 40 (by default) or 72.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Aligns the readout to the right of the available width.
    pub fn align_right(mut self, align_right: bool) -> Self {
        self.align_right = align_right;
        self
    }

    /// Without a glow, for calm contexts. Only orange, red and green have one.
    pub fn glow(mut self, glow: bool) -> Self {
        self.glow = glow;
        self
    }
}

fn tone_glow(tone: Tone) -> Option<Glow> {
    match tone {
        Tone::Accent => Some(tokens::GLOW_ORANGE),
        Tone::Alert => Some(tokens::GLOW_RED),
        Tone::Ok => Some(tokens::GLOW_GREEN),
        Tone::Warn | Tone::Info | Tone::Unit | Tone::Neutral => None,
    }
}

impl Widget for Readout<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let font_size = match self.size {
            Size::Sm => 24.0,
            Size::Md => 40.0,
            Size::Lg => 72.0,
        };
        let color = self.tone.color();
        let painter = ui.painter();
        let label =
            (self.label.is_some() || self.jp.is_some()).then(|| painter.layout_job(fonts::bilingual_label(self.label.unwrap_or_default(), self.jp)));
        let value = painter.layout_job(fonts::mono(self.value, font_size, color));
        let unit = self.unit.map(|unit| painter.layout_job(fonts::mono(unit, (font_size * 0.35).max(12.0), tokens::FG_2)));
        let detail = self.detail.map(|detail| painter.layout_job(fonts::text(detail, crate::Voice::Mono, tokens::TEXT_XS, 0.04, tokens::FG_3)));

        // `line-height: 1`: the line of the value is exactly as high as the font size.
        let value_width = value.size().x + unit.as_ref().map_or(0.0, |unit| UNIT_GAP + unit.size().x);
        let lines = [label.as_ref().map(|galley| galley.size()), Some(vec2(value_width, font_size)), detail.as_ref().map(|galley| galley.size())];
        let width = lines.iter().flatten().map(|size| size.x).fold(0.0, f32::max);
        let height = lines.iter().flatten().map(|size| size.y).sum::<f32>() + LINE_GAP * (lines.iter().flatten().count().saturating_sub(1)) as f32;

        let allocated_width = if self.align_right { ui.available_width().max(width) } else { width };
        let (rect, response) = ui.allocate_exact_size(vec2(allocated_width, height), Sense::hover());
        let name = format!("{} {} {}", self.label.unwrap_or_default().to_uppercase(), self.value, self.unit.unwrap_or_default());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, name.trim()));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let left = |line_width: f32| if self.align_right { rect.right() - line_width } else { rect.left() };
        let mut y = rect.top();
        if let Some(label) = &label {
            painter.galley(egui::pos2(left(label.size().x), y), label.clone(), tokens::FG_2);
            y += label.size().y + LINE_GAP;
        }
        // The value is centred in its line; the unit shares its baseline.
        let base = y + (font_size - value.size().y) / 2.0 + baseline(&value);
        let x = left(value_width);
        if self.glow
            && let Some(glow) = tone_glow(self.tone)
        {
            effects::text_glow(painter, position_on_line(x, base, &value), &value, glow);
        }
        let x = paint_on_line(painter, x, base, &value);
        if let Some(unit) = &unit {
            paint_on_line(painter, x + UNIT_GAP, base, unit);
        }
        y += font_size + LINE_GAP;
        if let Some(detail) = &detail {
            painter.galley(egui::pos2(left(detail.size().x), y), detail.clone(), tokens::FG_3);
        }
        response
    }
}
