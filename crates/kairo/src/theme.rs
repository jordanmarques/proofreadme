//! The Kairo egui theme: near-black background, radius 0, 1 px hairlines, no shadow, cyan focus.

use egui::epaint::Shadow;
use egui::style::{Selection, WidgetVisuals};
use egui::{CornerRadius, FontId, Stroke, Style, TextStyle, Visuals, vec2};

use crate::fonts::{self, Voice};
use crate::tokens;

/// Installs Kairo in the egui context: the embedded fonts, then the theme.
///
/// To be called once, at startup, before the first frame (with eframe: in the creation closure).
/// The fonts only become active at the start of the next frame, and a Kairo widget drawn without
/// them makes egui panic ("FontFamily … is not bound to any fonts"): where nothing can be prepared
/// before the first frame, go through [`ready`].
pub fn install(ctx: &egui::Context) {
    fonts::install(ctx);
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_style_of(egui::Theme::Dark, style());
    ctx.set_style_of(egui::Theme::Light, style());
}

/// True if Kairo is active in this context. Otherwise installs it, requests a new frame and
/// returns false: the caller then draws no Kairo widget during this frame.
///
/// ```no_run
/// # fn screen(ui: &mut egui::Ui) {
/// if !kairo::ready(ui) {
///     return;
/// }
/// ui.add(kairo::Button::primary("Launch"));
/// # }
/// ```
pub fn ready(ui: &egui::Ui) -> bool {
    let active = ui.ctx().fonts(|fonts| fonts.definitions().families.contains_key(&Voice::Display.family()));
    if !active {
        install(ui.ctx());
        ui.ctx().request_repaint();
    }
    active
}

/// The Kairo egui style. Assumes the fonts are installed (see [`install`]).
pub fn style() -> Style {
    let mut style = Style { visuals: visuals(), ..Style::default() };

    style.text_styles = [
        (TextStyle::Heading, Voice::LabelStrong.font(tokens::TEXT_XL)),
        (TextStyle::Body, Voice::Body.font(tokens::TEXT_MD)),
        (TextStyle::Button, Voice::Label.font(14.0)),
        (TextStyle::Small, Voice::Body.font(tokens::TEXT_XS)),
        (TextStyle::Monospace, FontId::new(tokens::TEXT_SM, egui::FontFamily::Monospace)),
    ]
    .into();

    style.spacing.item_spacing = vec2(tokens::SPACE_4, tokens::SPACE_4);
    style.spacing.button_padding = vec2(18.0, 0.0);
    style.spacing.interact_size = vec2(tokens::CONTROL_H_MD, tokens::CONTROL_H_MD);
    style.spacing.window_margin = egui::Margin::same(16);
    style.spacing.menu_margin = egui::Margin::same(8);
    // A solid, thin scroll bar, visible as soon as the content overflows: nothing floats or
    // appears on hover.
    style.spacing.scroll = egui::style::ScrollStyle { bar_width: 6.0, bar_inner_margin: 6.0, ..egui::style::ScrollStyle::solid() };

    // Mechanical, not organic: hard cuts, none of egui's fades.
    style.animation_time = 0.0;
    style
}

fn visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    let hairline = |color| Stroke::new(tokens::BORDER_HAIRLINE, color);
    let no_radius = CornerRadius::ZERO;

    visuals.override_text_color = None;
    visuals.panel_fill = tokens::BG_BASE;
    visuals.window_fill = tokens::BG_SURFACE;
    visuals.window_stroke = hairline(tokens::LINE_1);
    visuals.window_corner_radius = no_radius;
    visuals.menu_corner_radius = no_radius;
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;
    visuals.faint_bg_color = tokens::BG_SUNKEN;
    visuals.extreme_bg_color = tokens::BG_SUNKEN;
    visuals.code_bg_color = tokens::BG_SUNKEN;
    visuals.hyperlink_color = tokens::ACCENT;
    visuals.warn_fg_color = tokens::STATUS_WARN;
    visuals.error_fg_color = tokens::STATUS_ALERT;
    visuals.text_cursor.stroke = Stroke::new(tokens::BORDER_STRONG, tokens::ACCENT);
    // The focus is cyan. The text selection takes the muted orange: egui cannot invert the colour
    // of the selected text the way `::selection` does in CSS.
    visuals.selection = Selection { bg_fill: tokens::ACCENT_MUTED, stroke: hairline(tokens::FOCUS_RING) };
    visuals.disabled_alpha = 0.35;

    // The egui widgets Kairo does not redraw take the look of the "ghost" button.
    let base = WidgetVisuals {
        bg_fill: tokens::BG_SURFACE,
        weak_bg_fill: tokens::BG_SURFACE,
        bg_stroke: hairline(tokens::LINE_1),
        corner_radius: no_radius,
        fg_stroke: hairline(tokens::FG_1),
        expansion: 0.0,
    };
    visuals.widgets.noninteractive = base;
    visuals.widgets.inactive =
        WidgetVisuals { bg_fill: tokens::BG_RAISED, weak_bg_fill: egui::Color32::TRANSPARENT, bg_stroke: hairline(tokens::LINE_2), ..base };
    visuals.widgets.hovered =
        WidgetVisuals { bg_fill: tokens::K_INK_700, weak_bg_fill: tokens::K_INK_800, bg_stroke: hairline(tokens::FG_1), ..base };
    visuals.widgets.active = visuals.widgets.hovered;
    visuals.widgets.open = WidgetVisuals { bg_fill: tokens::BG_RAISED, weak_bg_fill: tokens::K_INK_800, bg_stroke: hairline(tokens::ACCENT), ..base };
    visuals
}
