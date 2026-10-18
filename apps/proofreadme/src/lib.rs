//! The ProofReadMe application: the window of the agent.
//!
//! This crate assembles the core (`proofreadme-core`) and the Kairo theme (`kairo`). The window
//! shows the state of the agent and passes the commands on to it, dressed with nothing but the
//! Kairo widgets and tokens. For now it opens empty, on the name of the agent.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

/// The name of the application, shown in the title of the window.
const NAME: &str = "ProofReadMe";

/// The window of the agent.
#[derive(Debug, Default)]
pub struct Window {}

impl Window {
    /// Creates the window of the agent.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The Kairo fonts are only active at the frame that follows their installation: until
        // then, nothing is drawn.
        if !kairo::ready(ui) {
            return;
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(kairo::tokens::BG_BASE)).show(ui, |ui| {
            ui.add(kairo::TitleCard::new("PROOFREADME").kicker("AGENT 校正").height(240.0).title_size(kairo::tokens::TEXT_4XL));
        });
    }
}

/// The options for opening the window: its title, its size, and on macOS the bearing of a session
/// agent (see [`no_dock`]).
#[must_use]
pub fn window_options() -> eframe::NativeOptions {
    /// The side, in pixels, of the icon given to the window on Windows.
    const ICON_SIDE: u32 = 256;

    let mut viewport = egui::ViewportBuilder::default().with_title(NAME).with_inner_size([760.0, 620.0]).with_min_inner_size([760.0, 460.0]);
    // On Windows the window carries the icon of the application (taskbar, Alt-Tab): without this
    // eframe puts its own there. On macOS the icon is that of the bundle.
    if cfg!(windows) {
        let icon = egui::IconData { rgba: proofreadme_icon::application(ICON_SIDE, 0.0), width: ICON_SIDE, height: ICON_SIDE };
        viewport = viewport.with_icon(icon);
    }
    eframe::NativeOptions { viewport, event_loop_builder: Some(Box::new(no_dock)), ..Default::default() }
}

/// macOS: the agent has neither a Dock icon nor a menu bar of its own ("accessory" activation
/// policy), which also holds outside any bundle.
///
/// The default winit menu is removed with it: its "Quit" (Cmd+Q) would end the process without
/// going through the request to quit of the agent.
#[cfg(target_os = "macos")]
fn no_dock(event_loop: &mut winit::event_loop::EventLoopBuilder<eframe::UserEvent>) {
    use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

    event_loop.with_activation_policy(ActivationPolicy::Accessory).with_default_menu(false);
}

/// Elsewhere, nothing to set: on Windows a hidden window leaves the taskbar by itself.
#[cfg(not(target_os = "macos"))]
fn no_dock<T>(_event_loop: &mut T) {}

/// What can prevent ProofReadMe from starting.
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    /// The window could not open.
    #[error("the window could not open: {0}")]
    Window(#[from] eframe::Error),
}

/// Opens the window of the agent, dressed with Kairo, and returns when it closes.
pub fn run() -> Result<(), LaunchError> {
    eframe::run_native(
        NAME,
        window_options(),
        Box::new(|creation| {
            kairo::install(&creation.egui_ctx);
            Ok(Box::new(Window::new()))
        }),
    )?;
    Ok(())
}
