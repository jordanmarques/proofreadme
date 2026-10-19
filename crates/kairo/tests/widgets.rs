//! The Kairo widgets, driven headless with `egui_kittest`.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kairo::gallery::Gallery;
use kairo::{Badge, Button, HazardStripe, Meter, Panel, Readout, StatusLight, Switch, TitleCard, Tone, Voice};

/// A harness where Kairo is installed, as in an application.
fn harness<'a>(mut ui: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    let mut harness = Harness::builder().with_size([900.0, 600.0]).build_ui(move |interface| {
        if kairo::ready(interface) {
            ui(interface);
        }
    });
    harness.run();
    harness
}

/// A pointer click at a precise place.
fn click_at(harness: &mut Harness<'_>, position: egui::Pos2) {
    harness.drag_at(position);
    harness.drop_at(position);
    harness.run();
}

// --- Button ---------------------------------------------------------------------------------

#[test]
fn a_click_on_a_button_is_reported() {
    let mut clicks = 0;
    {
        let mut harness = harness(|ui| {
            if ui.add(Button::primary("Authorize")).clicked() {
                clicks += 1;
            }
        });

        harness.get_by_label("AUTHORIZE").click();
        harness.run();
    }

    assert_eq!(clicks, 1);
}

#[test]
fn a_disabled_button_does_not_react_to_a_click() {
    let mut clicks = 0;
    {
        let mut harness = harness(|ui| {
            if ui.add(Button::primary("Authorize").enabled(false)).clicked() {
                clicks += 1;
            }
        });

        harness.get_by_label("AUTHORIZE").click();
        harness.run();
    }

    assert_eq!(clicks, 0);
}

#[test]
fn a_button_in_a_disabled_area_does_not_react_to_a_click() {
    let mut clicks = 0;
    {
        let mut harness = harness(|ui| {
            ui.add_enabled_ui(false, |ui| {
                if ui.add(Button::ghost("Close")).clicked() {
                    clicks += 1;
                }
            });
        });

        harness.get_by_label("CLOSE").click();
        harness.run();
    }

    assert_eq!(clicks, 0);
}

#[test]
fn every_variant_and_every_size_of_button_is_drawn() {
    let mut harness = harness(|ui| {
        ui.add(Button::primary("One").code("01"));
        ui.add(Button::secondary("Two").size(kairo::Size::Sm));
        ui.add(Button::ghost("Three").size(kairo::Size::Lg).full_width(true));
        ui.add(Button::danger("Four").frozen(kairo::Aspect::Hover));
        ui.add(Button::primary("Five").frozen(kairo::Aspect::Pressed));
    });
    harness.run();

    for label in ["ONE", "TWO", "THREE", "FOUR", "FIVE"] {
        harness.get_by_label(label);
    }
}

// --- Switch ---------------------------------------------------------------------------------

#[test]
fn a_click_on_a_switch_that_is_off_turns_it_on() {
    let mut on = false;
    let mut changes = 0;
    {
        let mut harness = harness(|ui| {
            if ui.add(Switch::new(&mut on).label("Automatic sync").jp("自動")).changed() {
                changes += 1;
            }
        });

        harness.get_by_label("AUTOMATIC SYNC").click();
        harness.run();
    }

    assert!(on);
    assert_eq!(changes, 1);
}

#[test]
fn a_second_click_on_a_switch_turns_it_off() {
    let mut on = false;
    {
        let mut harness = harness(|ui| {
            ui.add(Switch::new(&mut on).label("Automatic sync"));
        });

        harness.get_by_label("AUTOMATIC SYNC").click();
        harness.run();
        harness.get_by_label("AUTOMATIC SYNC").click();
        harness.run();
    }

    assert!(!on);
}

#[test]
fn a_click_on_the_already_picked_cell_of_a_switch_changes_nothing() {
    let mut on = false;
    let mut changes = 0;
    {
        let mut harness = harness(|ui| {
            if ui.add(Switch::new(&mut on).label("Automatic sync")).changed() {
                changes += 1;
            }
        });
        let rect = harness.get_by_label("AUTOMATIC SYNC").rect();
        // The OFF cell opens the widget, the ON cell follows it: each is at least 42 wide.
        let cell_off = rect.left_center() + egui::vec2(20.0, 0.0);
        let cell_on = rect.left_center() + egui::vec2(65.0, 0.0);

        click_at(&mut harness, cell_off);
        assert_eq!(harness.get_by_label("AUTOMATIC SYNC").accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));

        click_at(&mut harness, cell_on);
        click_at(&mut harness, cell_on);
    }

    assert!(on);
    assert_eq!(changes, 1);
}

#[test]
fn a_disabled_switch_does_not_react_to_a_click() {
    let mut on = false;
    {
        let mut harness = harness(|ui| {
            ui.add(Switch::new(&mut on).label("Automatic sync").enabled(false));
        });

        harness.get_by_label("AUTOMATIC SYNC").click();
        harness.run();
    }

    assert!(!on);
}

// --- Rendering of each widget ---------------------------------------------------------------

#[test]
fn a_panel_shows_its_title_and_its_content() {
    let mut harness = harness(|ui| {
        let response = Panel::new().title("Volumes").jp("媒体").code("VOL-01").tone(Tone::Alert).show(ui, |ui| {
            ui.label("Memory card");
            7
        });
        assert_eq!(response.inner, 7);
        Panel::new().brackets(false).margin(0.0).show(ui, |ui| ui.label("No header"));
    });
    harness.run();

    harness.get_by_label("Memory card");
    harness.get_by_label("No header");
}

#[test]
fn a_meter_is_drawn_and_announces_its_filled_share() {
    let mut harness = harness(|ui| {
        ui.add(Meter::new(50.0, 200.0).label("Progress").jp("進捗"));
        ui.add(Meter::new(3.0, 0.0).label("No maximum").tone(Tone::Accent).segments(0).show_value(false));
    });
    harness.run();

    assert_eq!(harness.get_by_label("PROGRESS").accesskit_node().numeric_value(), Some(0.25));
    assert_eq!(harness.get_by_label("NO MAXIMUM").accesskit_node().numeric_value(), Some(0.0));
}

#[test]
fn a_readout_is_drawn_in_every_size() {
    let mut harness = harness(|ui| {
        ui.add(Readout::new("87.4").label("Sync").jp("同期率").unit("%").size(kairo::Size::Lg).detail("▲ 2.1"));
        ui.add(Readout::new("128").label("Copied").tone(Tone::Accent).align_right(true));
        ui.add(Readout::new("003").tone(Tone::Alert).size(kairo::Size::Sm).glow(false));
    });
    harness.run();

    harness.get_by_label("SYNC 87.4 %");
    harness.get_by_label("COPIED 128");
    harness.get_by_label("003");
}

#[test]
fn a_status_light_is_drawn_in_every_state() {
    let mut harness = harness(|ui| {
        ui.add(StatusLight::ok().label("Ready"));
        ui.add(StatusLight::warn().label("Caution"));
        ui.add(StatusLight::alert().label("Alert").blink(true));
        ui.add(StatusLight::info().label("Reading").side(12.0));
        ui.add(StatusLight::off());
    });
    harness.run();

    for label in ["READY", "CAUTION", "ALERT", "READING"] {
        harness.get_by_label(label);
    }
}

#[test]
fn a_blinking_status_light_does_not_block_the_drawing_loop() {
    let mut harness = harness(|ui| {
        ui.add(StatusLight::alert().label("Alert").blink(true));
    });

    // The blink wakes up every half beat: never a continuous repaint.
    assert!(harness.run() < 5);
}

#[test]
fn a_badge_is_drawn_in_every_tone() {
    let tones = [Tone::Neutral, Tone::Accent, Tone::Ok, Tone::Warn, Tone::Alert, Tone::Info, Tone::Unit];
    let mut harness = harness(|ui| {
        for (index, tone) in tones.into_iter().enumerate() {
            ui.add(Badge::new(&format!("vol-{index}")).tone(tone).solid(index % 2 == 0));
        }
    });
    harness.run();

    for index in 0..tones.len() {
        harness.get_by_label(&format!("VOL-{index}"));
    }
}

#[test]
fn a_hazard_stripe_is_drawn_with_and_without_a_label() {
    let mut harness = harness(|ui| {
        ui.add(HazardStripe::orange());
        ui.add(HazardStripe::amber().height(28.0).label("Caution"));
        ui.add(HazardStripe::red().height(28.0).label("Restricted area"));
    });
    harness.run();

    harness.get_by_label("CAUTION");
    harness.get_by_label("RESTRICTED AREA");
}

#[test]
fn a_pattern_drawn_over_an_endless_or_undefined_area_does_not_loop() {
    harness(|ui| {
        let nowhere = egui::Rect::from_min_max(egui::pos2(f32::NAN, f32::NAN), egui::pos2(f32::NAN, f32::NAN));
        let oversized = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1e30, 1e30));
        for area in [egui::Rect::EVERYTHING, egui::Rect::NOTHING, nowhere, oversized] {
            kairo::effects::scanlines(ui.painter(), area);
            kairo::effects::grid(ui.painter(), area);
            kairo::effects::hazard_stripes(ui.painter(), area, kairo::tokens::K_ORANGE_500);
        }
    });
}

#[test]
fn a_title_card_is_drawn_complete_or_reduced_to_its_title() {
    let mut harness = harness(|ui| {
        ui.add(TitleCard::new("System\nwake-up").kicker("CHAPTER 01").episode("第壱話").subtitle("Power on").height(300.0));
        ui.add(TitleCard::new("Standing by").align_right(true).height(120.0).title_size(kairo::tokens::TEXT_3XL));
    });
    harness.run();

    harness.get_by_label("System wake-up");
    harness.get_by_label("Standing by");
}

// --- Gallery and fonts ----------------------------------------------------------------------

#[test]
fn the_gallery_shows_all_the_widgets_and_its_test_button_counts_the_clicks() {
    let mut gallery = Gallery::default();
    {
        let mut harness = Harness::builder().with_size(kairo::gallery::SIZE).build_ui(|ui| {
            if kairo::ready(ui) {
                gallery.show(ui);
            }
        });
        harness.run();

        harness.get_by_label("TEST").click();
        harness.run();
        harness.get_by_label("AUTO EJECT").click();
        harness.run();
    }

    assert_eq!(gallery.clicks, 1);
    assert!(gallery.switch_off);
}

/// The fingerprint of the glyph drawn for a character: its place in the glyph atlas.
fn glyph(ctx: &egui::Context, voice: Voice, character: char) -> Option<([u16; 2], [u16; 2])> {
    let galley = ctx.fonts_mut(|fonts| fonts.layout_no_wrap(character.to_string(), voice.font(32.0), egui::Color32::WHITE));
    let glyph = galley.rows.first()?.row.glyphs.first()?;
    Some((glyph.uv_rect.min, glyph.uv_rect.max))
}

#[test]
fn the_embedded_fonts_can_write_japanese_and_accented_latin() {
    let harness = harness(|_| {});
    let ctx = &harness.ctx;

    for voice in [Voice::Display, Voice::Label, Voice::LabelStrong, Voice::Mono, Voice::Body, Voice::Jp] {
        // A private-use character is in no font: it gives the replacement glyph.
        let replacement = glyph(ctx, voice, '\u{E000}');
        for character in "飛脚同期率カタカナひらがな第壱話ÉàçêùABCxyz0123456789%".chars() {
            let drawn = glyph(ctx, voice, character);
            assert!(drawn.is_some_and(|(min, max)| min != max), "{voice:?} draws nothing for '{character}'");
            assert_ne!(drawn, replacement, "{voice:?} has no glyph for '{character}'");
        }
    }
    // The geometric glyphs of Kairo, in mono.
    let replacement = glyph(ctx, Voice::Mono, '\u{E000}');
    for character in "■□▲▼◆●○→※".chars() {
        assert_ne!(glyph(ctx, Voice::Mono, character), replacement, "the mono has no glyph for '{character}'");
    }
}

#[test]
fn the_theme_has_no_rounded_corner_and_no_shadow_and_its_background_is_almost_black() {
    let harness = harness(|_| {});

    let style = harness.ctx.global_style();
    let visuals = &style.visuals;
    assert_eq!(visuals.panel_fill, egui::Color32::from_rgb(0x05, 0x05, 0x06));
    assert_eq!(visuals.window_corner_radius, egui::CornerRadius::ZERO);
    assert_eq!(visuals.menu_corner_radius, egui::CornerRadius::ZERO);
    assert_eq!(visuals.window_shadow, egui::epaint::Shadow::NONE);
    assert_eq!(visuals.popup_shadow, egui::epaint::Shadow::NONE);
    for widget in [visuals.widgets.noninteractive, visuals.widgets.inactive, visuals.widgets.hovered, visuals.widgets.active, visuals.widgets.open] {
        assert_eq!(widget.corner_radius, egui::CornerRadius::ZERO);
        assert_eq!(widget.bg_stroke.width, 1.0);
    }
    // Cyan focus.
    assert_eq!(visuals.selection.stroke.color, egui::Color32::from_rgb(0x33, 0xe4, 0xff));
}
