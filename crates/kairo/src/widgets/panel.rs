//! `Panel`: the bracketed instrument panel, the default container for any grouped content
//! (`components/layout/Panel.jsx`).

use egui::{Color32, CornerRadius, Frame, InnerResponse, Margin, Rect, Sense, Stroke, Ui, pos2, vec2};

use super::{Tone, paint_on_line};
use crate::fonts::{self, Voice};
use crate::{effects, tokens};

const HEADER_HEIGHT: f32 = 30.0;
const HEADER_MARGIN: f32 = 10.0;
const HEADER_GAP: f32 = 10.0;
const SQUARE: f32 = 8.0;
const CODE_LETTER_SPACING: f32 = 0.06;

/// The Kairo panel: surface background, a one-rule border, corner brackets, and a header of 30
/// with a coloured square, a title in capitals, a Japanese dressing and a code on the right. The
/// tone colours the square, the brackets and the code. It takes up the whole available width.
///
/// ```no_run
/// # fn screen(ui: &mut egui::Ui) {
/// kairo::Panel::new().title("Sensors").jp("計測").code("CAP-01").show(ui, |ui| {
///     ui.label("…");
/// });
/// # }
/// ```
#[must_use = "a Panel does nothing until it is shown: `.show(ui, ...)`"]
pub struct Panel<'a> {
    title: Option<&'a str>,
    jp: Option<&'a str>,
    code: Option<&'a str>,
    tone: Tone,
    brackets: bool,
    margin: f32,
    fill: Color32,
}

impl Default for Panel<'_> {
    fn default() -> Self {
        Self { title: None, jp: None, code: None, tone: Tone::Accent, brackets: true, margin: tokens::SPACE_6, fill: tokens::BG_SURFACE }
    }
}

impl<'a> Panel<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// The title of the header, put in capitals. With no title, no header.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// The Japanese dressing that follows the title.
    pub fn jp(mut self, jp: &'a str) -> Self {
        self.jp = Some(jp);
        self
    }

    /// The code on the right of the header (`SYS-04`).
    pub fn code(mut self, code: &'a str) -> Self {
        self.code = Some(code);
        self
    }

    /// The tone of the square, the brackets and the code: `Tone::Alert` turns the panel red.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// With or without corner brackets (with, by default).
    pub fn brackets(mut self, brackets: bool) -> Self {
        self.brackets = brackets;
        self
    }

    /// The inner margin of the body, 16 by default.
    pub fn margin(mut self, margin: f32) -> Self {
        self.margin = margin;
        self
    }

    /// The background of the panel, `BG_SURFACE` by default.
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }

    /// Shows the panel and its content.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let color = self.tone.color();
        let rule = Stroke::new(tokens::BORDER_HAIRLINE, tokens::LINE_1);
        let spacing = ui.spacing().item_spacing;
        let response = Frame::new().fill(self.fill).stroke(rule).inner_margin(Margin::ZERO).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.set_min_width(ui.available_width());
            if let Some(title) = self.title {
                self.header(ui, title, color, rule);
            }
            let margin = self.margin.round().clamp(0.0, 127.0) as i8;
            Frame::new()
                .inner_margin(Margin::same(margin))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = spacing;
                    ui.set_min_width(ui.available_width());
                    content(ui)
                })
                .inner
        });
        if self.brackets {
            effects::brackets(ui.painter(), response.response.rect, color);
        }
        response
    }

    fn header(&self, ui: &mut Ui, title: &str, color: Color32, rule: Stroke) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEADER_HEIGHT), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return;
        }
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        painter.hline(rect.x_range(), rect.bottom() - 0.5, rule);

        let center = rect.center().y;
        let mut x = rect.left() + HEADER_MARGIN;
        painter.rect_filled(Rect::from_min_size(pos2(x, center - SQUARE / 2.0), vec2(SQUARE, SQUARE)), CornerRadius::ZERO, color);
        x += SQUARE + HEADER_GAP;

        let title = painter.layout_job(fonts::text(title.to_uppercase(), Voice::LabelStrong, tokens::TEXT_SM, tokens::TRACKING_LABEL, tokens::FG_1));
        // The whole header shares the baseline of the title, itself centred in the 30 of height.
        let base = center - title.size().y / 2.0 + super::baseline(&title);
        x = paint_on_line(&painter, x, base, &title);
        if let Some(jp) = self.jp {
            let jp = painter.layout_job(fonts::jp(jp, tokens::TEXT_XS, tokens::FG_3));
            // The letter spacing of the title already leaves a gap after its last capital.
            paint_on_line(&painter, x + HEADER_GAP - tokens::TRACKING_LABEL * tokens::TEXT_SM, base, &jp);
        }
        if let Some(code) = self.code {
            let code = painter.layout_job(fonts::text(code, Voice::Mono, tokens::TEXT_XS, CODE_LETTER_SPACING, color));
            paint_on_line(&painter, rect.right() - HEADER_MARGIN - code.size().x, base, &code);
        }
    }
}
