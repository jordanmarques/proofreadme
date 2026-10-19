//! `StatusLight`: the square indicator light, with its label (`components/core/StatusLight.jsx`).

use egui::{Color32, CornerRadius, Rect, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

use crate::fonts::Voice;
use crate::tokens::{self, Glow};
use crate::{effects, fonts};

/// The state a light signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Light {
    #[default]
    Ok,
    Warn,
    Alert,
    Info,
    /// Off: no glow, no useful blinking.
    Off,
}

impl Light {
    fn color(self) -> Color32 {
        match self {
            Self::Ok => tokens::K_PHOSPHOR_500,
            Self::Warn => tokens::K_AMBER_500,
            Self::Alert => tokens::K_RED_500,
            Self::Info => tokens::K_CYAN_500,
            Self::Off => tokens::K_INK_600,
        }
    }
}

const GAP: f32 = 8.0;
/// `box-shadow: 0 0 8px <color>`.
const BLUR: f32 = 8.0;

/// The Kairo square indicator light. Only make it blink for what demands attention.
#[must_use = "a widget does nothing until it is added to the interface: `ui.add(...)`"]
pub struct StatusLight<'a> {
    state: Light,
    label: Option<&'a str>,
    jp: Option<&'a str>,
    blink: bool,
    side: f32,
}

impl<'a> StatusLight<'a> {
    pub fn new(state: Light) -> Self {
        Self { state, label: None, jp: None, blink: false, side: 8.0 }
    }

    pub fn ok() -> Self {
        Self::new(Light::Ok)
    }

    pub fn warn() -> Self {
        Self::new(Light::Warn)
    }

    pub fn alert() -> Self {
        Self::new(Light::Alert)
    }

    pub fn info() -> Self {
        Self::new(Light::Info)
    }

    pub fn off() -> Self {
        Self::new(Light::Off)
    }

    /// The label, put in capitals.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// The Japanese dressing that follows the label, smaller and paler (`QUEUED 待機`). With no
    /// label, it is not shown.
    pub fn jp(mut self, jp: &'a str) -> Self {
        self.jp = Some(jp);
        self
    }

    /// One-second blinking, in steps: on then off, with no fade.
    pub fn blink(mut self, blink: bool) -> Self {
        self.blink = blink;
        self
    }

    /// The side of the square, 8 by default.
    pub fn side(mut self, side: f32) -> Self {
        self.side = side;
        self
    }
}

impl Widget for StatusLight<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let ink = if self.state == Light::Off { tokens::FG_3 } else { tokens::FG_2 };
        let label = self.label.map(str::to_uppercase);
        let galley = label.as_deref().map(|label| {
            let mut job = fonts::label(label, 12.0, ink);
            if let Some(jp) = self.jp {
                // The gap of 8 between the two, minus the letter spacing that already follows the last capital.
                let gap = 8.0 - tokens::TRACKING_LABEL * 12.0;
                fonts::append_after(&mut job, gap, jp, Voice::Jp, tokens::TEXT_XS, tokens::TRACKING_JP, tokens::FG_3);
            }
            ui.painter().layout_job(job)
        });
        let width = self.side + galley.as_ref().map_or(0.0, |galley| GAP + galley.size().x);
        let height = galley.as_ref().map_or(self.side, |galley| galley.size().y.max(self.side));
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, label.as_deref().unwrap_or_default()));

        let on = !self.blink || effects::blink(ui);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let square = Rect::from_min_size(pos2(rect.left(), rect.center().y - self.side / 2.0), vec2(self.side, self.side));
            if on {
                let color = self.state.color();
                if self.state != Light::Off {
                    effects::glow(painter, square, Glow { blur: BLUR, color });
                }
                painter.rect_filled(square, CornerRadius::ZERO, color);
            }
            if let Some(galley) = galley {
                painter.galley(pos2(square.right() + GAP, rect.center().y - galley.size().y / 2.0), galley, ink);
            }
        }
        response
    }
}
