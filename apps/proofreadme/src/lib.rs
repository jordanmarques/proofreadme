//! The ProofReadMe application: the window of the agent.
//!
//! This crate assembles the core (`proofreadme-core`) and the Kairo theme (`kairo`). The window
//! shows the state of the agent and passes the commands on to it, dressed with nothing but the
//! Kairo widgets and tokens. Two screens: the home of the agent and its settings.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod keychain;
mod settings;
mod unwired;

use std::path::PathBuf;

use kairo::{Button, StatusLight, TitleCard, tokens};
use proofreadme_core::{Activity, Agent, Settings, Storage, Timing};

pub use keychain::SystemKeychain;
pub use unwired::UnwiredOs;

/// The name of the application, shown in the title of the window.
const NAME: &str = "ProofReadMe";

/// The screen shown by the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// The name and the activity of the agent.
    Home,
    /// The settings form.
    Settings,
}

/// What the user asked for on a screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// Open the settings.
    OpenSettings,
    /// Save the settings being edited.
    Save,
    /// Test the provider with the settings being edited.
    Test,
    /// Back to the home screen.
    Back,
}

/// The window of the agent.
pub struct Window {
    agent: Agent,
    screen: Screen,
    /// The settings being edited, taken from the agent when the settings screen opens.
    draft: Settings,
    /// The requests of the agent for the settings already followed.
    followed_requests: u64,
    /// True since the last save succeeded, until the settings are edited again.
    saved: bool,
}

impl Window {
    /// Creates the window of an agent. It opens on the settings whenever the agent asks for them,
    /// at a first launch included.
    #[must_use]
    pub fn new(agent: Agent) -> Self {
        let draft = agent.settings();
        Self { agent, screen: Screen::Home, draft, followed_requests: 0, saved: false }
    }

    /// The agent of the window.
    #[must_use]
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    fn open_settings(&mut self) {
        self.screen = Screen::Settings;
        self.draft = self.agent.settings();
        self.saved = false;
    }

    fn act(&mut self, action: Action) {
        match action {
            Action::OpenSettings => self.open_settings(),
            Action::Save => self.saved = self.agent.save_settings(self.draft.clone()).is_ok(),
            // A refusal of the Test lands in the state of the agent, which the screen shows.
            Action::Test => {
                let _ = self.agent.test_provider(&self.draft);
            }
            Action::Back => self.screen = Screen::Home,
        }
    }
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let state = self.agent.state();
        if state.settings_requests > self.followed_requests {
            self.followed_requests = state.settings_requests;
            self.open_settings();
        }
        // The Kairo fonts are only active at the frame that follows their installation: until
        // then, nothing is drawn.
        if !kairo::ready(ui) {
            return;
        }
        let before = self.draft.clone();
        let mut action = None;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(tokens::BG_BASE).inner_margin(tokens::SPACE_6)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(tokens::SPACE_5, tokens::SPACE_5);
                action = match self.screen {
                    Screen::Home => home(ui, &state),
                    Screen::Settings => settings::show(ui, &mut self.draft, &state, self.saved),
                };
            });
        });
        if self.draft != before {
            self.saved = false;
        }
        if let Some(action) = action {
            self.act(action);
        }
    }
}

/// The home screen: the name of the agent, what it is doing, and the way to the settings.
fn home(ui: &mut egui::Ui, state: &proofreadme_core::State) -> Option<Action> {
    ui.add(TitleCard::new("PROOFREADME").kicker("AGENT 校正").height(240.0).title_size(tokens::TEXT_4XL));
    let light = match state.activity {
        Activity::Idle => StatusLight::ok().label("Listening").jp("待機"),
        Activity::Proofreading(_) => StatusLight::info().blink(true).label("Proofreading").jp("校正中"),
    };
    ui.add(light);
    ui.add(Button::secondary("Settings").jp("設定")).clicked().then_some(Action::OpenSettings)
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
    /// The OS gives no data folder to keep the config in.
    #[error("the OS gives no data folder for the config")]
    NoDataFolder,
}

/// The location of the config: `config.toml` in the data folder of the application.
fn config_path() -> Result<PathBuf, LaunchError> {
    dirs::data_local_dir().map(|folder| folder.join(NAME).join("config.toml")).ok_or(LaunchError::NoDataFolder)
}

/// Opens the window of the agent, dressed with Kairo, and returns when it closes.
pub fn run() -> Result<(), LaunchError> {
    let storage = Storage::new(config_path()?, SystemKeychain);
    eframe::run_native(
        NAME,
        window_options(),
        Box::new(|creation| {
            kairo::install(&creation.egui_ctx);
            let context = creation.egui_ctx.clone();
            let agent = Agent::load(UnwiredOs, storage, Timing::default()).on_change(move || context.request_repaint());
            Ok(Box::new(Window::new(agent)))
        }),
    )?;
    Ok(())
}
