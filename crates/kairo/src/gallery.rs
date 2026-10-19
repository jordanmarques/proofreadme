//! The demonstration gallery: every Kairo widget in its variants and states, to judge the
//! rendering by eye against the HTML cards of the design system.
//!
//! It lives in the crate so that it serves both the runnable example
//! (`cargo run -p kairo --example gallery`) and the tests.

use egui::{Ui, vec2};

use crate::fonts::{self, Voice};
use crate::{Aspect, Badge, Button, HazardStripe, Meter, Panel, Readout, Size, StatusLight, Switch, TitleCard, Tone, effects, tokens};

/// The recommended size to see the whole gallery, in points.
pub const SIZE: egui::Vec2 = vec2(1180.0, 1700.0);

/// What the gallery remembers from one frame to the next.
#[derive(Debug, Clone, PartialEq)]
pub struct Gallery {
    /// The number of clicks received by the test button.
    pub clicks: u32,
    pub switch_off: bool,
    pub switch_on: bool,
    pub switch_manual: bool,
    switch_disabled: bool,
    /// The test gauge, driven by the "less" and "more" buttons.
    pub progress: f32,
    /// With or without scanlines over the screen.
    pub scanlines: bool,
}

/// The constructor of a Button variant.
type Variant = fn(&'static str) -> Button<'static>;

impl Default for Gallery {
    fn default() -> Self {
        Self { clicks: 0, switch_off: false, switch_on: true, switch_manual: true, switch_disabled: true, progress: 67.0, scanlines: true }
    }
}

impl Gallery {
    /// Draws the gallery. Assumes Kairo is installed in the context (see [`crate::install`]).
    pub fn show(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(tokens::SPACE_6, tokens::SPACE_6);
        ui.add(
            TitleCard::new("Widget\ngallery")
                .kicker("KAIRO 回路 // EGUI")
                .episode("第壱話")
                .subtitle("Every widget, every state")
                .height(380.0)
                .title_size(tokens::TEXT_4XL),
        );
        ui.add(HazardStripe::orange());

        ui.columns(2, |columns| {
            if let [left, right] = columns {
                for column in [&mut *left, &mut *right] {
                    column.spacing_mut().item_spacing = vec2(tokens::SPACE_6, tokens::SPACE_6);
                }
                self.buttons(left);
                self.switches(left);
                badges(left);
                lights(left);
                self.meters(right);
                readouts(right);
                alert(right);
                voices(right);
            }
        });

        ui.add(HazardStripe::amber().height(28.0).label("Caution 注意"));
        ui.add(HazardStripe::red().height(28.0).label("Restricted area 立入禁止"));
        if self.scanlines {
            effects::fullscreen_scanlines(ui.ctx());
        }
    }

    fn buttons(&mut self, ui: &mut Ui) {
        Panel::new().title("Button").jp("操作").code("BTN-01").show(ui, |ui| {
            let variants: [(&str, Variant); 4] =
                [("primary", Button::primary), ("secondary", Button::secondary), ("ghost", Button::ghost), ("danger", Button::danger)];
            egui::Grid::new("gallery-buttons").spacing(vec2(tokens::SPACE_5, tokens::SPACE_5)).show(ui, |ui| {
                ui.label("");
                for state in ["Rest", "Hover", "Pressed", "Disabled"] {
                    ui.label(fonts::label(state, 12.0, tokens::FG_3));
                }
                ui.end_row();
                for (name, variant) in variants {
                    ui.label(fonts::mono(name, tokens::TEXT_XS, tokens::FG_3));
                    ui.add(variant("Launch").frozen(Aspect::Rest));
                    ui.add(variant("Launch").frozen(Aspect::Hover));
                    ui.add(variant("Launch").frozen(Aspect::Pressed));
                    ui.add(variant("Launch").enabled(false));
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                ui.add(Button::primary("Small").size(Size::Sm));
                ui.add(Button::primary("Medium").code("02"));
                ui.add(Button::secondary("Large").size(Size::Lg).code("03"));
            });
            ui.horizontal(|ui| {
                if ui.add(Button::primary("Test").code("01")).clicked() {
                    self.clicks += 1;
                }
                ui.label(fonts::mono(format!("CLICKS {:03}", self.clicks), tokens::TEXT_SM, tokens::K_PHOSPHOR_500));
            });
            ui.horizontal(|ui| {
                ui.add(Button::primary("Authorize").jp("許可"));
                ui.add(Button::ghost("Ignore").jp("無視").size(Size::Sm));
            });
            ui.add(Button::ghost("Full width").full_width(true));
        });
    }

    fn switches(&mut self, ui: &mut Ui) {
        Panel::new().title("Switch").jp("切替").code("SWT-02").show(ui, |ui| {
            ui.add(Switch::new(&mut self.switch_off).label("Auto eject").jp("排出"));
            ui.add(Switch::new(&mut self.switch_on).label("Auto sync").jp("自動"));
            ui.add(Switch::new(&mut self.switch_manual).label("Mode").jp("方式").texts("MAN", "AUTO"));
            ui.add(Switch::new(&mut self.switch_disabled).label("Disabled").jp("無効").enabled(false));
        });
    }

    fn meters(&mut self, ui: &mut Ui) {
        Panel::new().title("Meter").jp("計器").code("MTR-05").show(ui, |ui| {
            ui.add(Meter::new(32.0, 100.0).label("Idle").jp("低"));
            ui.add(Meter::new(self.progress, 100.0).label("Progress").jp("進捗"));
            ui.add(Meter::new(96.0, 100.0).label("Saturation").jp("飽和"));
            ui.add(Meter::new(128.0, 412.0).label("Copy").jp("複写").tone(Tone::Accent).segments(40).readout("128 / 412"));
            ui.add(Meter::new(100.0, 100.0).label("Alert").jp("警告").tone(Tone::Alert).height(6.0).show_value(false));
            ui.horizontal(|ui| {
                if ui.add(Button::ghost("Less").size(Size::Sm)).clicked() {
                    self.progress = (self.progress - 5.0).max(0.0);
                }
                if ui.add(Button::ghost("More").size(Size::Sm)).clicked() {
                    self.progress = (self.progress + 5.0).min(100.0);
                }
            });
        });
    }
}

fn badges(ui: &mut Ui) {
    const TONES: [(Tone, &str); 7] = [
        (Tone::Neutral, "VOL-01"),
        (Tone::Accent, "SYN-02"),
        (Tone::Ok, "OK"),
        (Tone::Warn, "WRN-04"),
        (Tone::Alert, "ALR-03"),
        (Tone::Info, "INF-07"),
        (Tone::Unit, "N-01"),
    ];
    Panel::new().title("Badge").jp("標識").code("BDG-03").show(ui, |ui| {
        for solid in [false, true] {
            ui.horizontal_wrapped(|ui| {
                for (tone, text) in TONES {
                    ui.add(Badge::new(text).tone(tone).solid(solid));
                }
            });
        }
    });
}

fn lights(ui: &mut Ui) {
    Panel::new().title("StatusLight").jp("表示灯").code("LED-04").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = tokens::SPACE_7;
            ui.add(StatusLight::ok().label("Ready"));
            ui.add(StatusLight::warn().label("Caution"));
            ui.add(StatusLight::alert().label("Alert").blink(true));
            ui.add(StatusLight::info().label("Reading").jp("読取"));
            ui.add(StatusLight::off().label("Off"));
        });
    });
}

fn readouts(ui: &mut Ui) {
    Panel::new().title("Readout").jp("数値").code("RDT-06").show(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = tokens::SPACE_8;
            ui.add(Readout::new("87.4").label("Sync").jp("同期率").unit("%").size(Size::Lg).detail("▲ 2.1 / MIN"));
            ui.add(Readout::new("128").label("Copied").jp("複写").unit("/ 412").tone(Tone::Accent));
        });
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = tokens::SPACE_8;
            ui.add(Readout::new("003").label("Errors").jp("異常").tone(Tone::Alert));
            ui.add(Readout::new("12").label("Duplicates").jp("重複").tone(Tone::Info).size(Size::Sm));
            ui.add(Readout::new("284").label("Calm").jp("静").unit("GB").glow(false).size(Size::Sm));
        });
    });
}

fn alert(ui: &mut Ui) {
    Panel::new().title("Panel in alert").jp("警告").code("ALR-03").tone(Tone::Alert).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add(StatusLight::alert().label("Interrupted").blink(true));
            ui.add(Badge::new("ALR-03").tone(Tone::Alert).solid(true));
        });
        ui.label(fonts::body("Link lost during the transfer. Three items were not received.", tokens::TEXT_MD, tokens::FG_1));
        ui.label(fonts::body("※ Restore the link then relaunch.", tokens::TEXT_SM, tokens::FG_3));
        ui.horizontal(|ui| {
            ui.add(Button::danger("Abort"));
            ui.add(Button::ghost("Close"));
        });
    });
    Panel::new().brackets(false).margin(tokens::SPACE_5).show(ui, |ui| {
        ui.label(fonts::label("No header, no brackets", 12.0, tokens::FG_2));
    });
}

fn voices(ui: &mut Ui) {
    Panel::new().title("Voices").jp("書体").code("TYP-07").tone(Tone::Neutral).show(ui, |ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), effects::display_line_height(tokens::TEXT_3XL)), egui::Sense::hover());
        effects::display(ui.painter(), rect.left_bottom(), "起動 Mincho", tokens::TEXT_3XL, tokens::K_PAPER);
        ui.label(fonts::text("CONDENSED 700 — TITLES", Voice::LabelStrong, tokens::TEXT_LG, tokens::TRACKING_LABEL, tokens::FG_1));
        ui.label(fonts::bilingual_label("Condensed 600, labels", Some("ラベル")));
        ui.label(fonts::mono("MONO 0123456789 VOL-01 ■ □ ▲ ▼ ◆ ● ○ → ※", tokens::TEXT_SM, tokens::K_PHOSPHOR_500));
        ui.label(fonts::body("Body in Barlow: running text reads in normal case, without capitals.", tokens::TEXT_MD, tokens::FG_1));
        ui.label(fonts::jp("起動を確認しました。カタカナ ひらがな 漢字", tokens::TEXT_MD, tokens::FG_2));
    });
}
