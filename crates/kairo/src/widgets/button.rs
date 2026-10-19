//! `Button`: the rectangular command button (`components/core/Button.jsx`).

use egui::{Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

use super::{Aspect, Size};
use crate::effects;
use crate::fonts::{self, Voice};
use crate::tokens::{self, Glow};

/// The colours of a variant, at rest and on hover.
struct Variant {
    fill: Color32,
    text: Color32,
    border: Color32,
    hover_fill: Color32,
    hover_border: Option<Color32>,
    hover_glow: Option<Glow>,
}

const PRIMARY: Variant = Variant {
    fill: tokens::K_ORANGE_500,
    text: tokens::K_VOID,
    border: tokens::K_ORANGE_500,
    hover_fill: tokens::K_ORANGE_400,
    hover_border: None,
    hover_glow: Some(tokens::GLOW_ORANGE),
};
const SECONDARY: Variant = Variant {
    fill: Color32::TRANSPARENT,
    text: tokens::K_ORANGE_500,
    border: tokens::K_ORANGE_500,
    hover_fill: tokens::K_ORANGE_900,
    hover_border: None,
    hover_glow: None,
};
const GHOST: Variant = Variant {
    fill: Color32::TRANSPARENT,
    text: tokens::FG_1,
    border: tokens::LINE_2,
    hover_fill: tokens::K_INK_800,
    hover_border: Some(tokens::FG_1),
    hover_glow: None,
};
const DANGER: Variant = Variant {
    fill: tokens::K_RED_500,
    text: tokens::K_PAPER,
    border: tokens::K_RED_500,
    hover_fill: tokens::K_RED_700,
    hover_border: None,
    hover_glow: Some(tokens::GLOW_RED),
};

/// The opacity of a disabled button.
const DISABLED_OPACITY: f32 = 0.35;
/// The gap between the label and its Japanese dressing.
const JP_GAP: f32 = 8.0;
/// The opacity of the Japanese dressing, paler than the label.
const JP_OPACITY: f32 = 0.65;
/// The gap between the code and the label, on either side of the rule that separates them.
const CODE_GAP: f32 = 10.0;

/// The Kairo command button. The label is put in capitals.
///
/// ```no_run
/// # fn screen(ui: &mut egui::Ui) {
/// if ui.add(kairo::Button::primary("Authorize").code("01")).clicked() {
///     // …
/// }
/// # }
/// ```
#[must_use = "a widget does nothing until it is added to the interface: `ui.add(...)`"]
pub struct Button<'a> {
    label: &'a str,
    variant: &'static Variant,
    size: Size,
    code: Option<&'a str>,
    jp: Option<&'a str>,
    full_width: bool,
    enabled: bool,
    frozen: Option<Aspect>,
}

impl<'a> Button<'a> {
    fn new(label: &'a str, variant: &'static Variant) -> Self {
        Self { label, variant, size: Size::Md, code: None, jp: None, full_width: false, enabled: true, frozen: None }
    }

    /// Solid orange: the single main action of the screen.
    pub fn primary(label: &'a str) -> Self {
        Self::new(label, &PRIMARY)
    }

    /// Orange outline: a secondary action.
    pub fn secondary(label: &'a str) -> Self {
        Self::new(label, &SECONDARY)
    }

    /// Grey outline: a discreet action.
    pub fn ghost(label: &'a str) -> Self {
        Self::new(label, &GHOST)
    }

    /// Solid red: a destructive action.
    pub fn danger(label: &'a str) -> Self {
        Self::new(label, &DANGER)
    }

    /// The size of the button: 28, 36 (by default) or 48 high.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// A mono index before the label, separated by a rule (`01`).
    pub fn code(mut self, code: &'a str) -> Self {
        self.code = Some(code);
        self
    }

    /// The Japanese dressing that follows the label, smaller and paler (`AUTHORIZE 許可`).
    pub fn jp(mut self, jp: &'a str) -> Self {
        self.jp = Some(jp);
        self
    }

    /// The button takes up the whole available width.
    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// A disabled button is dimmed and does not react to a click.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Freezes the aspect of the button, whatever the pointer does: to show the hover or the press
    /// in a gallery or a capture.
    pub fn frozen(mut self, aspect: Aspect) -> Self {
        self.frozen = Some(aspect);
        self
    }
}

impl Widget for Button<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (height, margin, font_size) = match self.size {
            Size::Sm => (tokens::CONTROL_H_SM, 12.0, 12.0),
            Size::Md => (tokens::CONTROL_H_MD, 18.0, 14.0),
            Size::Lg => (tokens::CONTROL_H_LG, 26.0, 17.0),
        };
        let variant = self.variant;
        let label = self.label.to_uppercase();
        let painter = ui.painter();
        let text = painter.layout_job(fonts::label(&label, font_size, variant.text));
        let code = self.code.map(|code| painter.layout_job(fonts::text(code, Voice::Mono, font_size * 0.8, 0.0, variant.text.gamma_multiply(0.75))));
        let code_width = code.as_ref().map_or(0.0, |code| code.size().x + CODE_GAP + tokens::BORDER_HAIRLINE + CODE_GAP);
        let jp = self.jp.map(|jp| painter.layout_job(fonts::jp(jp, font_size * 0.8, variant.text.gamma_multiply(JP_OPACITY))));
        // The letter spacing of the label already leaves a gap after its last capital.
        let jp_gap = JP_GAP - tokens::TRACKING_LABEL * font_size;
        let jp_width = jp.as_ref().map_or(0.0, |jp| jp_gap + jp.size().x);
        let content = code_width + text.size().x + jp_width;
        let width = if self.full_width { ui.available_width().max(content + 2.0 * margin) } else { content + 2.0 * margin };

        let enabled = self.enabled && ui.is_enabled();
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), if enabled { Sense::click() } else { Sense::hover() });
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &label));

        if !ui.is_rect_visible(rect) {
            return response;
        }
        let aspect = match self.frozen {
            Some(aspect) => aspect,
            None if !enabled => Aspect::Rest,
            None if response.is_pointer_button_down_on() => Aspect::Pressed,
            None if response.hovered() => Aspect::Hover,
            None => Aspect::Rest,
        };
        let hovered = aspect != Aspect::Rest;
        // Press: `translateY(1px)`, a hard cut.
        let rect = if aspect == Aspect::Pressed { rect.translate(vec2(0.0, 1.0)) } else { rect };

        let mut painter = ui.painter().clone();
        // A disabled parent already dims what it paints: do not dim twice.
        if !self.enabled && ui.is_enabled() {
            painter.multiply_opacity(DISABLED_OPACITY);
        }
        if hovered && let Some(glow) = variant.hover_glow {
            effects::glow(&painter, rect, glow);
        }
        let fill = if hovered { variant.hover_fill } else { variant.fill };
        let border = if hovered { variant.hover_border.unwrap_or(variant.border) } else { variant.border };
        painter.rect(rect, CornerRadius::ZERO, fill, Stroke::new(tokens::BORDER_HAIRLINE, border), StrokeKind::Inside);

        let mut x = rect.center().x - content / 2.0;
        if let Some(code) = &code {
            painter.galley(pos2(x, rect.center().y - code.size().y / 2.0), code.clone(), variant.text);
            x += code.size().x + CODE_GAP;
            let half = code.size().y / 2.0;
            painter.vline(
                x + 0.5,
                (rect.center().y - half)..=(rect.center().y + half),
                Stroke::new(tokens::BORDER_HAIRLINE, variant.text.gamma_multiply(0.75)),
            );
            x += tokens::BORDER_HAIRLINE + CODE_GAP;
        }
        let text_end = x + text.size().x;
        painter.galley(pos2(x, rect.center().y - text.size().y / 2.0), text, variant.text);
        if let Some(jp) = jp {
            painter.galley(pos2(text_end + jp_gap, rect.center().y - jp.size().y / 2.0), jp, variant.text);
        }

        if response.has_focus() {
            effects::focus_ring(&painter, rect);
        }
        response
    }
}
