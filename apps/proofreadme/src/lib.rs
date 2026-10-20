//! The ProofReadMe application: the window of the agent.
//!
//! This crate assembles the core (`proofreadme-core`) and the Kairo theme (`kairo`). The window
//! shows the state of the agent and passes the commands on to it, dressed with nothing but the
//! Kairo widgets and tokens. A single screen: the settings of the agent.
//!
//! The agent lives in the session (ADR 0001): a session icon shows its state, a second launch
//! brings its settings window back, closing the window hides it, and it registers itself at login.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod icon;
mod instance;
mod keychain;
mod notifications;
mod permission;
mod platform;
mod session;
mod settings;
mod startup;
mod unsupported;

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use kairo::tokens;
use proofreadme_core::{Agent, AgentLog, IconState, LoginItem, Settings, Storage, Timing};

pub use icon::{IconError, IconPort, SessionIcon};
pub use instance::{Instance, InstanceListener, announce};
pub use keychain::SystemKeychain;
pub use notifications::SystemNotifier;
pub use session::{Request, RequestSender};
pub use startup::{BACKGROUND_ARGUMENT, IDENTIFIER, LoginRegistration, RegistrationError, SystemRegistration, system_registration};
pub use unsupported::UnsupportedOs;

/// The name of the application, shown in the title of the window.
const NAME: &str = "ProofReadMe";

/// What the user asked for on a screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// Save the settings being edited.
    Save,
    /// Test the provider with the settings being edited.
    Test,
    /// Open the pane of System Settings where the permission of the keyboard hook is granted.
    OpenPermissionPane,
}

/// The look the session icon was last given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Look {
    icon: IconState,
    can_cancel: bool,
}

/// The window of the agent.
pub struct Window {
    agent: Agent,
    /// The settings being edited, taken from the agent again whenever it asks for its settings.
    draft: Settings,
    /// The requests of the agent for the settings already followed.
    followed_requests: u64,
    /// The requests of the agent for its window already followed.
    followed_window_requests: u64,
    /// True since the last save succeeded, until the settings are edited again.
    saved: bool,
    presence: session::Presence,
    /// The registration at login, which executes what the agent asks of it; `None` remembers the
    /// preference without registering anything.
    registration: Option<Box<dyn LoginRegistration>>,
    /// The session icon, and the look it was last given.
    icon: Option<Box<dyn IconPort>>,
    shown_look: Option<Look>,
    /// Opens the pane of System Settings where the permission is granted.
    permission_pane: Option<Box<dyn FnMut()>>,
}

impl Window {
    /// Creates the window of an agent. It opens on the settings whenever the agent asks for them,
    /// at a first launch included.
    #[must_use]
    pub fn new(agent: Agent) -> Self {
        let draft = agent.settings();
        Self {
            agent,
            draft,
            followed_requests: 0,
            followed_window_requests: 0,
            saved: false,
            presence: session::Presence::new(),
            registration: None,
            icon: None,
            shown_look: None,
            permission_pane: None,
        }
    }

    /// Gives the window the port of registration at login, which executes what the agent asks of
    /// it: at first launch, the agent registers.
    #[must_use]
    pub fn with_login_registration(mut self, registration: impl LoginRegistration + 'static) -> Self {
        self.registration = Some(Box::new(registration));
        self.follow_login_item();
        self
    }

    /// Gives the window the requests that come to it from elsewhere: the session icon, a second
    /// launch of the application (see [`RequestSender`]).
    #[must_use]
    pub fn with_requests(mut self, requests: Receiver<Request>) -> Self {
        self.presence.listen(requests);
        self
    }

    /// Gives the window the session icon, which it keeps in step with the state of the agent.
    #[must_use]
    pub fn with_session_icon(mut self, icon: impl IconPort + 'static) -> Self {
        self.icon = Some(Box::new(icon));
        self
    }

    /// Gives the window the way to open the pane of System Settings where the permission of the
    /// keyboard hook is granted: the permission panel offers it while the permission is missing.
    #[must_use]
    pub fn with_permission_pane(mut self, open: impl FnMut() + 'static) -> Self {
        self.permission_pane = Some(Box::new(open));
        self
    }

    /// Launches the agent in the background: the window stays hidden as long as the agent does not
    /// need the user and nobody asks to open it.
    #[must_use]
    pub fn in_background(mut self) -> Self {
        self.presence.start_in_background();
        self
    }

    /// Tells the window that the agent has no session icon: hiding it would make it impossible to
    /// find, and nothing would allow quitting it any more. Closing the window then quits the agent,
    /// and the settings say so.
    #[must_use]
    pub fn without_session_icon(mut self) -> Self {
        self.presence.without_session_icon();
        self
    }

    /// True since the window asked to be hidden (closed by the user, launched in the background),
    /// until it asks to come back to the foreground.
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.presence.is_hidden()
    }

    /// Executes the registration at login the agent asks for, if any, and reports its outcome.
    fn follow_login_item(&mut self) {
        let Some(registration) = self.registration.as_mut() else { return };
        let Some(item) = self.agent.take_login_item() else { return };
        let outcome = match item {
            LoginItem::Register => registration.register(),
            LoginItem::Unregister => registration.unregister(),
        };
        match outcome {
            Ok(()) => self.agent.login_item_done(item, Ok(())),
            Err(cause) => self.agent.login_item_done(item, Err(&cause)),
        }
    }

    /// Handles what is asked of the window from the session icon or a second launch.
    fn handle_requests(&mut self, context: &egui::Context) {
        for request in self.presence.received_requests() {
            match request {
                Request::Open => self.presence.show(context),
                Request::OpenSettings => {
                    self.open_settings();
                    self.presence.show(context);
                }
                Request::Cancel => self.agent.cancel(),
                Request::Quit => self.presence.quit(context),
            }
        }
    }

    /// Gives the session icon the look of the state, when it changed.
    fn update_icon(&mut self, state: &proofreadme_core::State) {
        let look = Look { icon: state.icon(), can_cancel: state.can_cancel() };
        if let Some(icon) = self.icon.as_mut()
            && self.shown_look != Some(look)
        {
            icon.show(look.icon, look.can_cancel);
            self.shown_look = Some(look);
        }
    }

    /// The agent of the window.
    #[must_use]
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    fn open_settings(&mut self) {
        self.draft = self.agent.settings();
        self.saved = false;
    }

    fn act(&mut self, action: Action) {
        match action {
            Action::Save => self.saved = self.agent.save_settings(self.draft.clone()).is_ok(),
            // A refusal of the Test lands in the state of the agent, which the screen shows.
            Action::Test => {
                let _ = self.agent.test_provider(&self.draft);
            }
            Action::OpenPermissionPane => {
                if let Some(open) = self.permission_pane.as_mut() {
                    open();
                }
            }
        }
    }
}

impl eframe::App for Window {
    /// The life of the agent, window shown or hidden: eframe calls `logic` before each frame, and
    /// alone when the window is hidden.
    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_requests(context);
        self.follow_login_item();
        let state = self.agent.state();
        let needs_the_settings = state.settings_requests > self.followed_requests;
        if needs_the_settings {
            self.followed_requests = state.settings_requests;
            self.open_settings();
        }
        let needs_the_window = state.window_requests > self.followed_window_requests;
        self.followed_window_requests = state.window_requests;
        self.presence.update(context, needs_the_settings || needs_the_window);
        self.update_icon(&state);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let state = self.agent.state();
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
                let pane = permission::show(ui, &state, self.permission_pane.is_some());
                action = settings::show(ui, &mut self.draft, &state, self.saved, self.presence.has_no_icon()).or(pane);
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

/// The data folder of the application.
fn data_folder() -> Result<PathBuf, LaunchError> {
    dirs::data_local_dir().map(|folder| folder.join(NAME)).ok_or(LaunchError::NoDataFolder)
}

/// The location of the config: `config.toml` in the data folder of the application.
fn config_path() -> Result<PathBuf, LaunchError> {
    data_folder().map(|folder| folder.join("config.toml"))
}

/// The location of the agent log, one line per failed proofread and per problem of a platform
/// adapter: `logs/agent.txt` in the data folder of the application.
fn log_path() -> Result<PathBuf, LaunchError> {
    data_folder().map(|folder| folder.join("logs").join("agent.txt"))
}

/// Runs the session agent and returns when it quits.
///
/// If an agent is already running in the session, its settings window is brought back to the
/// foreground and the function returns at once. Launched with [`BACKGROUND_ARGUMENT`] (which is
/// what the registration at login does), the agent starts with its window hidden.
pub fn run() -> Result<(), LaunchError> {
    let config = config_path()?;
    let log = AgentLog::new(log_path()?);
    let (sender, requests) = RequestSender::new();
    // Nothing of what follows prevents the agent from starting: failing a single instance, an icon
    // or a registration, it stays usable, and the cause is told in the agent log or in the
    // settings.
    match config.parent().map(|folder| announce(folder, "agent")) {
        Some(Ok(Instance::AlreadyRunning)) => return Ok(()),
        Some(Ok(Instance::First(listener))) => {
            if let Err(error) = listener.listen(sender.clone()) {
                log.note(&format!("the single instance is not guaranteed: {error}"));
            }
        }
        Some(Err(error)) => log.note(&format!("the single instance is not guaranteed: {error}")),
        None => {}
    }
    let in_background = std::env::args().any(|argument| argument == BACKGROUND_ARGUMENT);
    let storage = Storage::new(config, SystemKeychain);
    eframe::run_native(
        NAME,
        window_options(),
        Box::new(move |creation| {
            kairo::install(&creation.egui_ctx);
            sender.wake(creation.egui_ctx.clone());
            let context = creation.egui_ctx.clone();
            let agent = Agent::load(platform::os(), storage, Timing::default())
                .with_notifier(SystemNotifier::new(log.clone()))
                .with_log(log.clone())
                .on_change(move || context.request_repaint());
            platform::listen(&agent);
            let mut window = Window::new(agent.clone()).with_requests(requests);
            if let Some(open) = platform::permission_pane(&agent) {
                window = window.with_permission_pane(open);
            }
            match system_registration() {
                Ok(Some(registration)) => window = window.with_login_registration(registration),
                Ok(None) => {}
                Err(cause) => agent.login_unavailable(&cause),
            }
            // The icon is created here: on the main thread, with the event loop already started.
            match SessionIcon::install(&sender, log) {
                Ok(icon) => {
                    window = window.with_session_icon(icon);
                    if in_background {
                        window = window.in_background();
                    }
                }
                // Without an icon, a hidden agent would be impossible to find and could no longer
                // be quit: its window stays shown, even launched in the background, and closing it
                // quits it.
                Err(error) => {
                    agent.note(&format!("the session icon could not be installed: {error}. Closing the window quits ProofReadMe."));
                    window = window.without_session_icon();
                }
            }
            Ok(Box::new(window))
        }),
    )?;
    Ok(())
}
