//! The screens of ProofReadMe rendered as images, without opening a window.
//!
//! ```sh
//! cargo run -p proofreadme --example screens -- --capture <folder>
//! ```
//!
//! Each scene plugs the real window into the real agent, over a data folder simulated in a
//! temporary folder and an in-memory keychain, then saves a PNG. The session icons are drawn in
//! their three looks, as the menu bar of macOS and the notification area of Windows show them.
//! Nothing is read or written outside these temporary folders and the capture folder.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use proofreadme::{RegistrationError, UnsupportedOs, Window};
use proofreadme_core::{Agent, MemoryKeychain, Permission, Provider, Settings, Storage, Timing};
use proofreadme_icon::Look;

/// The size of the captured window, in points: the one the application opens at.
const SIZE: egui::Vec2 = egui::vec2(760.0, 620.0);
/// The density of the capture: two pixels per point, like a Retina screen.
const DENSITY: f32 = 2.0;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match arguments.as_slice() {
        [option, folder] if option == "--capture" => capture(Path::new(folder)),
        _ => Err("usage: screens --capture <folder>".to_owned()),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("screens: {error}");
            ExitCode::FAILURE
        }
    }
}

fn capture(folder: &Path) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(say)?;
    home(folder)?;
    permission_missing(folder)?;
    settings_at_first_launch(folder)?;
    settings_without_registration(folder)?;
    session_icons(folder)
}

fn say(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// An agent over a data folder simulated in `data`, configured with a provider if `configured`.
fn agent(data: &Path, configured: bool) -> Result<Agent, String> {
    let keychain = MemoryKeychain::with_key("sk-screens");
    let storage = || Storage::new(data.join("ProofReadMe").join("config.toml"), keychain.clone());
    if configured {
        let provider =
            Provider { base_url: "https://openrouter.ai/api/v1".to_owned(), api_key: "sk-screens".to_owned(), model: "vendor/model".to_owned() };
        Agent::load(UnsupportedOs, storage(), Timing::default()).save_settings(Settings { provider, ..Settings::default() }).map_err(say)?;
    }
    Ok(Agent::load(UnsupportedOs, storage(), Timing::default()))
}

fn open<'a>(window: Window) -> Harness<'a, Window> {
    let mut harness = Harness::builder().with_size(SIZE).with_pixels_per_point(DENSITY).wgpu().build_eframe(|creation| {
        kairo::install(&creation.egui_ctx);
        window
    });
    harness.run_ok();
    harness
}

fn save<State>(harness: &mut Harness<'_, State>, folder: &Path, name: &str) -> Result<(), String> {
    // The pointer of the clicks leaves the window: no hover and no cursor in the image.
    harness.input_mut().events.push(egui::Event::PointerGone);
    // A few frames so that the glyph atlas is complete before rendering, then until the blinking
    // lights are on.
    harness.run_steps(4);
    for _ in 0..16 {
        if kairo::effects::blink_is_on(harness.ctx.input(|input| input.time)) {
            break;
        }
        harness.step();
    }
    let image = harness.render()?;
    let path = folder.join(format!("{name}.png"));
    image.save(&path).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    println!("screens: {} ({} × {} px)", path.display(), image.width(), image.height());
    Ok(())
}

fn home(folder: &Path) -> Result<(), String> {
    let data = tempfile::tempdir().map_err(say)?;
    let mut harness = open(Window::new(agent(data.path(), true)?));
    save(&mut harness, folder, "home")
}

/// The home while the keyboard hook lacks the permission of the OS: the permission panel.
fn permission_missing(folder: &Path) -> Result<(), String> {
    let data = tempfile::tempdir().map_err(say)?;
    let agent = agent(data.path(), true)?;
    agent.hook().permission(Permission::Missing);
    let mut harness = open(Window::new(agent).with_permission_pane(|| {}));
    save(&mut harness, folder, "home-permission-missing")
}

fn settings_at_first_launch(folder: &Path) -> Result<(), String> {
    let data = tempfile::tempdir().map_err(say)?;
    let mut harness = open(Window::new(agent(data.path(), false)?));
    save(&mut harness, folder, "settings-first-launch")
}

/// The settings scrolled to the trigger panel, where the registration at login and the absence of
/// a session icon are told.
fn settings_without_registration(folder: &Path) -> Result<(), String> {
    let data = tempfile::tempdir().map_err(say)?;
    let refusal = RegistrationError::from_system("access denied");
    let agent = agent(data.path(), false)?;
    agent.login_unavailable(&refusal);
    let window = Window::new(agent).without_session_icon();
    let mut harness = open(window);
    harness.query_all_by_label("LAUNCH AT LOGIN").next().ok_or("no launch-at-login switch")?.scroll_to_me();
    harness.run_ok();
    save(&mut harness, folder, "settings-registration-refused")
}

/// The three looks of the session icon: on the light and dark menu bars of macOS (a template
/// tinted by the system, drawn here in black then white), in the notification area of Windows
/// (orange), then enlarged.
fn session_icons(folder: &Path) -> Result<(), String> {
    const LOOKS: [(Look, &str); 3] = [(Look::Idle, "IDLE"), (Look::Running, "RUNNING"), (Look::Alert, "ALERT")];
    /// The rows: name, background, colour of the drawing, side drawn in pixels, side shown in points.
    const ROWS: [(&str, egui::Color32, [u8; 3], u32, f32); 4] = [
        ("macOS light", egui::Color32::from_rgb(0xec, 0xec, 0xec), [0, 0, 0], 44, 22.0),
        ("macOS dark", egui::Color32::from_rgb(0x2b, 0x2b, 0x2b), [255, 255, 255], 44, 22.0),
        ("Windows", egui::Color32::from_rgb(0x20, 0x20, 0x20), proofreadme_icon::ORANGE, 32, 16.0),
        ("Enlarged", egui::Color32::from_rgb(0x2b, 0x2b, 0x2b), proofreadme_icon::ORANGE, 176, 88.0),
    ];

    let mut harness = Harness::builder().with_size(egui::vec2(480.0, 300.0)).with_pixels_per_point(DENSITY).wgpu().build_ui_state(
        |ui, textures: &mut Vec<egui::TextureHandle>| {
            if textures.is_empty() {
                for (row, (_, _, color, side, _)) in ROWS.iter().enumerate() {
                    for (look, _) in LOOKS {
                        let side = *side as usize;
                        let image = egui::ColorImage::from_rgba_unmultiplied([side, side], &proofreadme_icon::session(side as u32, *color, look));
                        textures.push(ui.ctx().load_texture(format!("{row}-{look:?}"), image, egui::TextureOptions::LINEAR));
                    }
                }
            }
            egui::Grid::new("icons").spacing([24.0, 12.0]).show(ui, |ui| {
                ui.label("");
                for (_, name) in LOOKS {
                    ui.label(name);
                }
                ui.end_row();
                for (row, (name, background, _, _, shown)) in ROWS.iter().enumerate() {
                    ui.label(*name);
                    for column in 0..LOOKS.len() {
                        if let Some(texture) = textures.get(row * LOOKS.len() + column) {
                            egui::Frame::new().fill(*background).inner_margin(6.0).show(ui, |ui| {
                                ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(*shown, *shown)));
                            });
                        }
                    }
                    ui.end_row();
                }
            });
        },
        Vec::new(),
    );
    harness.run_ok();
    save(&mut harness, folder, "session-icons")
}
