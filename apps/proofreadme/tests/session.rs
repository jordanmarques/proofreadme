//! The life of the session agent seen from the window, driven headless with `egui_kittest`.
//!
//! The window is plugged into the real agent. The session icon and the registration at login are
//! simulated ports that note what is asked of them: no test touches the menu bar, a LaunchAgent or
//! the registry. The requests of the menu and of a second launch arrive through the same channel as
//! in production. What the window asks of the OS (to hide, to come back, to close) is read in the
//! viewport commands of the last frame.

mod support;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use egui::{ViewportCommand, ViewportEvent, ViewportId};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use proofreadme::{IconPort, LoginRegistration, RegistrationError, Request, Window};
use proofreadme_core::{
    Agent, ClipboardContent, DEFAULT_TRIGGER, Direction, IconState, Key, KeyEvent, Keystroke, Mark, MemoryKeychain, Os, OsError, Permission,
    Settings, Storage, Timing,
};
use support::{API_KEY, DELAY, config, loaded, observed, show_until};

/// A configured agent over the data folder simulated in `folder`: no settings to ask for.
fn configured(folder: &std::path::Path, launch_at_login: bool) -> Agent {
    let keychain = MemoryKeychain::with_key(API_KEY);
    let settings = Settings { provider: support::provider("http://127.0.0.1:9/api/v1"), launch_at_login, ..Settings::default() };
    loaded(folder, &keychain).save_settings(settings).unwrap();
    loaded(folder, &keychain)
}

/// A simulated registration: it notes the calls received, and fails if asked to.
#[derive(Clone, Default)]
struct SimulatedRegistration {
    calls: Rc<RefCell<Vec<&'static str>>>,
    broken: bool,
}

impl SimulatedRegistration {
    fn calls(&self) -> Vec<&'static str> {
        self.calls.borrow().clone()
    }

    fn note(&self, call: &'static str) -> Result<(), RegistrationError> {
        self.calls.borrow_mut().push(call);
        if self.broken { Err(RegistrationError::from_system("access denied")) } else { Ok(()) }
    }
}

impl LoginRegistration for SimulatedRegistration {
    fn register(&mut self) -> Result<(), RegistrationError> {
        self.note("register")
    }

    fn unregister(&mut self) -> Result<(), RegistrationError> {
        self.note("unregister")
    }
}

/// A simulated session icon: it notes each look it is given, with the state of its Cancel item.
#[derive(Clone, Default)]
struct SimulatedIcon {
    looks: Rc<RefCell<Vec<(IconState, bool)>>>,
}

impl SimulatedIcon {
    fn looks(&self) -> Vec<(IconState, bool)> {
        self.looks.borrow().clone()
    }
}

impl IconPort for SimulatedIcon {
    fn show(&mut self, icon: IconState, can_cancel: bool) {
        self.looks.borrow_mut().push((icon, can_cancel));
    }
}

/// An OS whose copy is held until the test releases it, then refused: a proofread that runs as
/// long as the test wants, then fails.
struct HeldOs {
    release: Arc<Mutex<Receiver<()>>>,
}

impl Os for HeldOs {
    fn read_clipboard(&mut self) -> Result<ClipboardContent, OsError> {
        let _ = self.release.lock().unwrap().recv_timeout(DELAY);
        Err(OsError::Clipboard("refused by the test".to_owned()))
    }

    fn write_clipboard(&mut self, _content: &ClipboardContent, _mark: Mark) -> Result<(), OsError> {
        Ok(())
    }

    fn clipboard_change_count(&mut self) -> u64 {
        0
    }

    fn synthesize(&mut self, _keystroke: Keystroke) -> Result<(), OsError> {
        Ok(())
    }

    fn foreground_application(&mut self) -> Option<String> {
        None
    }
}

/// What the window asked of the OS during the last frame.
fn commands(window: &Harness<'_, Window>) -> Vec<ViewportCommand> {
    window.output().viewport_output.get(&ViewportId::ROOT).map(|output| output.commands.clone()).unwrap_or_default()
}

/// The user clicks the close box of the window.
fn request_the_close(window: &mut Harness<'_, Window>) {
    window.input_mut().viewports.get_mut(&ViewportId::ROOT).unwrap().events.push(ViewportEvent::Close);
}

fn is_checked(window: &Harness<'_, Window>, label: &str) -> bool {
    format!("{:?}", window.get_by_label(label).accesskit_node().toggled()).contains("True")
}

/// A double press of the default trigger key (Cmd on macOS, Ctrl elsewhere).
fn trigger(agent: &Agent) {
    let now = Instant::now();
    for direction in [Direction::Down, Direction::Up, Direction::Down, Direction::Up] {
        agent.key_event(KeyEvent { key: Key::Modifier(DEFAULT_TRIGGER), direction, at: now, injected: false });
    }
}

const SWITCH: &str = "LAUNCH AT LOGIN";

#[test]
fn the_session_icon_follows_the_agent_from_idle_to_running_and_back_after_a_failure() {
    let folder = tempfile::tempdir().unwrap();
    let keychain = MemoryKeychain::with_key(API_KEY);
    configured(folder.path(), false);
    let (release, held) = mpsc::channel::<()>();
    let os = HeldOs { release: Arc::new(Mutex::new(held)) };
    let (agent, wakeups) = observed(Agent::load(os, Storage::new(config(folder.path()), keychain), Timing::default()));
    let icon = SimulatedIcon::default();
    let port = icon.clone();
    let mut window = Harness::builder().build_eframe(|_| Window::new(agent).with_session_icon(port));
    window.run_ok();
    assert_eq!(icon.looks(), [(IconState::Idle, false)], "the icon is given its look once, at start");

    trigger(window.state().agent());
    show_until(&mut window, &wakeups, |_| icon.looks().last() == Some(&(IconState::Running, true)));
    release.send(()).unwrap();
    show_until(&mut window, &wakeups, |_| icon.looks().last() == Some(&(IconState::Idle, false)));

    assert_eq!(icon.looks(), [(IconState::Idle, false), (IconState::Running, true), (IconState::Idle, false)], "a failure is notified, not shown");
}

#[test]
fn a_permission_lost_gives_the_session_icon_the_alert_look() {
    let folder = tempfile::tempdir().unwrap();
    let (agent, wakeups) = observed(configured(folder.path(), false));
    let hook = agent.hook();
    let icon = SimulatedIcon::default();
    let port = icon.clone();
    let mut window = Harness::builder().build_eframe(|_| Window::new(agent).with_session_icon(port));
    window.run_ok();

    hook.permission(Permission::Missing);
    show_until(&mut window, &wakeups, |_| icon.looks().last() == Some(&(IconState::Alert, false)));
}

#[test]
fn a_second_launch_brings_the_hidden_window_back_to_the_foreground() {
    let folder = tempfile::tempdir().unwrap();
    let (sender, requests) = mpsc::channel();
    let mut window = Harness::builder().build_eframe(|_| Window::new(configured(folder.path(), false)).with_requests(requests));
    window.run_ok();
    request_the_close(&mut window);
    window.step();

    sender.send(Request::Open).unwrap();
    window.step();

    let commands = commands(&window);
    assert!(commands.contains(&ViewportCommand::Visible(true)), "the window is shown again: {commands:?}");
    assert!(commands.contains(&ViewportCommand::Focus), "the window comes to the foreground: {commands:?}");
    assert!(!window.state().is_hidden());
}

#[test]
fn settings_from_the_menu_opens_the_window_on_the_settings() {
    let folder = tempfile::tempdir().unwrap();
    let (sender, requests) = mpsc::channel();
    let mut window = Harness::builder().build_eframe(|_| Window::new(configured(folder.path(), false)).with_requests(requests));
    window.run_ok();
    assert!(window.query_by_label("SAVE").is_none(), "the window opens on the home screen");

    sender.send(Request::OpenSettings).unwrap();
    window.step();
    window.run_ok();

    window.get_by_label("SAVE");
}

#[test]
fn closing_the_window_hides_it_without_quitting_the_agent() {
    let folder = tempfile::tempdir().unwrap();
    let mut window = Harness::builder().build_eframe(|_| Window::new(configured(folder.path(), false)));
    window.run_ok();

    request_the_close(&mut window);
    window.step();

    let commands = commands(&window);
    assert!(commands.contains(&ViewportCommand::CancelClose), "the close is cancelled: {commands:?}");
    assert!(commands.contains(&ViewportCommand::Visible(false)), "the window is hidden: {commands:?}");
    assert!(window.state().is_hidden());
}

#[test]
fn quit_from_the_menu_closes_the_window_for_good() {
    let folder = tempfile::tempdir().unwrap();
    let (sender, requests): (Sender<Request>, _) = mpsc::channel();
    let mut window = Harness::builder().build_eframe(|_| Window::new(configured(folder.path(), false)).with_requests(requests));
    window.run_ok();

    sender.send(Request::Quit).unwrap();
    window.step();
    assert!(commands(&window).contains(&ViewportCommand::Close), "the close is requested");

    // The OS relays the close at the next frame: this time it is not cancelled.
    request_the_close(&mut window);
    window.step();
    assert!(!commands(&window).contains(&ViewportCommand::CancelClose));
}

#[test]
fn without_a_session_icon_closing_the_window_quits_the_agent() {
    let folder = tempfile::tempdir().unwrap();
    let mut window = Harness::builder().build_eframe(|_| Window::new(configured(folder.path(), false)).without_session_icon());
    window.run_ok();

    request_the_close(&mut window);
    window.step();

    let commands = commands(&window);
    assert!(!commands.contains(&ViewportCommand::CancelClose), "the close is not cancelled: {commands:?}");
    assert!(!window.state().is_hidden());
}

#[test]
fn launched_in_the_background_the_window_stays_hidden() {
    let folder = tempfile::tempdir().unwrap();

    // The harness draws several frames as soon as it is built: the window itself says what it
    // asked for.
    let window = Harness::builder().build_eframe(|_| Window::new(configured(folder.path(), false)).in_background());

    assert!(window.state().is_hidden());
}

#[test]
fn launched_in_the_background_at_first_launch_the_window_opens_on_the_settings() {
    let folder = tempfile::tempdir().unwrap();
    let agent = loaded(folder.path(), &MemoryKeychain::default());

    let mut window = Harness::builder().build_eframe(|_| Window::new(agent).in_background());
    window.run_ok();

    assert!(!window.state().is_hidden(), "the settings need the user");
    window.get_by_label("SAVE");
}

#[test]
fn a_trigger_without_a_provider_brings_the_hidden_window_back_on_the_settings() {
    let folder = tempfile::tempdir().unwrap();
    let keychain = MemoryKeychain::default();
    loaded(folder.path(), &keychain).save_settings(Settings::default()).unwrap();
    let (agent, wakeups) = observed(loaded(folder.path(), &keychain));
    let mut window = Harness::builder().build_eframe(|_| Window::new(agent).in_background());
    // A config without a key: the agent asks for the settings at launch, which shows the window;
    // the user closes it.
    window.run_ok();
    request_the_close(&mut window);
    window.step();
    assert!(window.state().is_hidden());

    trigger(window.state().agent());
    show_until(&mut window, &wakeups, |window| !window.state().is_hidden());

    window.get_by_label("SAVE");
}

#[test]
fn at_first_launch_the_switch_is_on_and_the_agent_registers() {
    let folder = tempfile::tempdir().unwrap();
    let registration = SimulatedRegistration::default();
    let port = registration.clone();
    let agent = loaded(folder.path(), &MemoryKeychain::default());

    let mut window = Harness::builder().build_eframe(|_| Window::new(agent).with_login_registration(port));
    window.run_ok();

    assert!(is_checked(&window, SWITCH));
    assert_eq!(registration.calls(), ["register"]);
}

#[test]
fn saving_the_switch_off_removes_the_registration() {
    let folder = tempfile::tempdir().unwrap();
    let registration = SimulatedRegistration::default();
    let port = registration.clone();
    let agent = configured(folder.path(), true);
    let mut window = Harness::builder().with_size(egui::vec2(760.0, 1600.0)).build_eframe(|_| Window::new(agent).with_login_registration(port));
    window.run_ok();
    window.get_by_label("SETTINGS").click();
    window.run_ok();

    window.get_by_label(SWITCH).click();
    window.run_ok();
    assert_eq!(registration.calls(), ["register"], "nothing changes before the save");
    window.get_by_label("SAVE").click();
    window.run_ok();

    assert_eq!(registration.calls(), ["register", "unregister"]);
    assert!(!loaded(folder.path(), &MemoryKeychain::with_key(API_KEY)).settings().launch_at_login);
}

#[test]
fn a_disabled_preference_removes_the_registration_at_launch() {
    let folder = tempfile::tempdir().unwrap();
    let registration = SimulatedRegistration::default();
    let port = registration.clone();
    let agent = configured(folder.path(), false);

    let mut window = Harness::builder().build_eframe(|_| Window::new(agent).with_login_registration(port));
    window.run_ok();

    assert_eq!(registration.calls(), ["unregister"]);
}

#[test]
fn a_failed_registration_is_told_in_the_settings() {
    let folder = tempfile::tempdir().unwrap();
    let registration = SimulatedRegistration { broken: true, ..SimulatedRegistration::default() };
    let agent = loaded(folder.path(), &MemoryKeychain::default());

    let mut window =
        Harness::builder().with_size(egui::vec2(760.0, 1600.0)).build_eframe(|_| Window::new(agent).with_login_registration(registration));
    window.run_ok();

    window.get_by_label_contains("access denied");
}

#[cfg(debug_assertions)]
#[test]
fn a_debug_build_never_registers_at_login() {
    assert!(proofreadme::system_registration().unwrap().is_none());
}
