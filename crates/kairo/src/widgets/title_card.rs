//! `TitleCard`: the black title card in heavy stretched mincho, for section openings, welcome
//! screens and empty states (`components/brand/TitleCard.jsx`).

use egui::{CornerRadius, Response, Sense, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2};

use crate::fonts::{self, Voice};
use crate::{effects, tokens};

const TOP_MARGIN: f32 = 40.0;
const SIDE_MARGIN: f32 = 48.0;
const GAP: f32 = 28.0;
const EPISODE_SIZE: f32 = 40.0;
const TITLE_SIZE: f32 = 88.0;
const KICKER_LETTER_SPACING: f32 = 0.24;

/// The Kairo title card: orange mono kicker at the top; at the bottom, episode and title in
/// stretched mincho, then subtitle in capitals. It takes up the whole available width.
///
/// ```no_run
/// # fn screen(ui: &mut egui::Ui) {
/// ui.add(kairo::TitleCard::new("Standing by").kicker("SYSTEM 起動").episode("待機"));
/// # }
/// ```
#[must_use = "a widget does nothing until it is added to the interface: `ui.add(...)`"]
pub struct TitleCard<'a> {
    title: &'a str,
    kicker: Option<&'a str>,
    episode: Option<&'a str>,
    subtitle: Option<&'a str>,
    align_right: bool,
    height: f32,
    title_size: f32,
}

impl<'a> TitleCard<'a> {
    /// A card 360 high. `\n` in the title breaks the line.
    pub fn new(title: &'a str) -> Self {
        Self { title, kicker: None, episode: None, subtitle: None, align_right: false, height: 360.0, title_size: TITLE_SIZE }
    }

    /// The orange mono kicker, at the top of the card (`kicker` in Kairo).
    pub fn kicker(mut self, kicker: &'a str) -> Self {
        self.kicker = Some(kicker);
        self
    }

    /// The episode line in mincho, above the title (`第壱話`).
    pub fn episode(mut self, episode: &'a str) -> Self {
        self.episode = Some(episode);
        self
    }

    /// The subtitle in spaced capitals, under the title.
    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    /// Aligns the card to the right.
    pub fn align_right(mut self, align_right: bool) -> Self {
        self.align_right = align_right;
        self
    }

    /// The height of the card, 360 by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// The font size of the title, 88 by default. Kairo fixes it at 88; a narrow window will take
    /// another value of the scale (`tokens::TEXT_3XL`, `tokens::TEXT_4XL`).
    pub fn title_size(mut self, size: f32) -> Self {
        self.title_size = size;
        self
    }
}

impl Widget for TitleCard<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, self.title.replace('\n', " ")));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        // `overflow: hidden`: nothing sticks out of the card.
        let painter = &ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        painter.rect_filled(rect, CornerRadius::ZERO, tokens::K_VOID);
        let inner = rect.shrink2(vec2(SIDE_MARGIN, TOP_MARGIN));
        let left = |width: f32| if self.align_right { inner.right() - width } else { inner.left() };

        if let Some(kicker) = self.kicker {
            let galley = painter.layout_job(fonts::text(kicker, Voice::Mono, tokens::TEXT_SM, KICKER_LETTER_SPACING, tokens::K_ORANGE_500));
            painter.galley(pos2(left(galley.size().x), inner.top()), galley, tokens::K_ORANGE_500);
        }

        // The bottom group is composed going up from the foot of the card.
        let mut bottom = inner.bottom();
        if let Some(subtitle) = self.subtitle {
            let galley =
                painter.layout_job(fonts::text(subtitle.to_uppercase(), Voice::Label, tokens::TEXT_LG, tokens::TRACKING_WIDE, tokens::K_PAPER_DIM));
            bottom -= galley.size().y;
            painter.galley(pos2(left(galley.size().x), bottom), galley, tokens::K_PAPER_DIM);
            bottom -= GAP;
        }
        for line in self.title.lines().rev() {
            let width = effects::display_width(painter, line, self.title_size);
            effects::display(painter, pos2(left(width), bottom), line, self.title_size, tokens::K_PAPER);
            bottom -= effects::display_line_height(self.title_size);
        }
        if let Some(episode) = self.episode {
            bottom -= GAP;
            let width = effects::display_width(painter, episode, EPISODE_SIZE);
            effects::display(painter, pos2(left(width), bottom), episode, EPISODE_SIZE, tokens::K_PAPER);
        }
        response
    }
}
