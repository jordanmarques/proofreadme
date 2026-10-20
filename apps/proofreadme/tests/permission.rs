//! The permission panel of the window, driven headless with `egui_kittest`: the permission the
//! keyboard hook needs is faked through the hook of the agent, and the opening of the pane of
//! System Settings is a simulated port that counts its calls.

mod support;

use std::cell::Cell;
use std::rc::Rc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use proofreadme::Window;
use proofreadme_core::{Agent, MemoryKeychain, Permission, Settings};
use support::{API_KEY, loaded, observed, show_until};

/// A configured agent over the data folder simulated in `folder`: no settings to ask for.
fn configured(folder: &std::path::Path) -> Agent {
    let keychain = MemoryKeychain::with_key(API_KEY);
    let settings = Settings { provider: support::provider("http://127.0.0.1:9/api/v1"), ..Settings::default() };
    loaded(folder, &keychain).save_settings(settings).unwrap();
    loaded(folder, &keychain)
}

/// A window over the agent whose pane of System Settings counts how many times it was opened.
fn window(agent: Agent) -> (Harness<'static, Window>, Rc<Cell<u32>>) {
    let opened = Rc::new(Cell::new(0));
    let counter = Rc::clone(&opened);
    let window = Harness::builder().build_eframe(move |creation| {
        kairo::install(&creation.egui_ctx);
        Window::new(agent).with_permission_pane(move || counter.set(counter.get() + 1))
    });
    (window, opened)
}

#[test]
fn a_granted_permission_shows_no_panel() {
    let folder = tempfile::tempdir().unwrap();
    let (mut window, _) = window(configured(folder.path()));

    window.run_ok();

    window.get_by_label("SAVE");
    assert!(window.query_by_label("OPEN SYSTEM SETTINGS").is_none());
}

#[test]
fn a_missing_permission_shows_the_panel_whose_button_opens_the_pane_of_system_settings() {
    let folder = tempfile::tempdir().unwrap();
    let agent = configured(folder.path());
    agent.hook().permission(Permission::Missing);
    let (mut window, opened) = window(agent);
    window.run_ok();

    window.get_by_label_contains("Accessibility");
    window.get_by_label("SAVE");
    window.get_by_label("OPEN SYSTEM SETTINGS").click();
    window.run_ok();

    assert_eq!(opened.get(), 1);
}

#[test]
fn a_permission_that_needs_a_relaunch_says_so() {
    let folder = tempfile::tempdir().unwrap();
    let agent = configured(folder.path());
    agent.hook().permission(Permission::RelaunchNeeded);
    let (mut window, _) = window(agent);

    window.run_ok();

    window.get_by_label_contains("Relaunch ProofReadMe");
    assert!(window.query_by_label("OPEN SYSTEM SETTINGS").is_none(), "the permission is already granted");
}

#[test]
fn the_panel_goes_away_once_the_permission_is_granted() {
    let folder = tempfile::tempdir().unwrap();
    let (agent, wakeups) = observed(configured(folder.path()));
    let hook = agent.hook();
    hook.permission(Permission::Missing);
    let (mut window, _) = window(agent);
    window.run_ok();
    window.get_by_label("OPEN SYSTEM SETTINGS");

    hook.permission(Permission::Granted);
    show_until(&mut window, &wakeups, |window| window.query_by_label("OPEN SYSTEM SETTINGS").is_none());
}

#[test]
fn a_permission_lost_brings_the_hidden_window_back() {
    let folder = tempfile::tempdir().unwrap();
    let (agent, wakeups) = observed(configured(folder.path()));
    let hook = agent.hook();
    let mut window = Harness::builder().build_eframe(move |creation| {
        kairo::install(&creation.egui_ctx);
        Window::new(agent).with_permission_pane(|| {}).in_background()
    });
    window.run_ok();
    assert!(window.state().is_hidden());

    hook.permission(Permission::Missing);
    show_until(&mut window, &wakeups, |window| !window.state().is_hidden());

    window.get_by_label("OPEN SYSTEM SETTINGS");
}

#[test]
fn a_hook_that_cannot_listen_brings_the_hidden_window_back_and_says_why() {
    let folder = tempfile::tempdir().unwrap();
    let (agent, wakeups) = observed(configured(folder.path()));
    let hook = agent.hook();
    let mut window = Harness::builder().build_eframe(move |creation| {
        kairo::install(&creation.egui_ctx);
        Window::new(agent).in_background()
    });
    window.run_ok();
    assert!(window.state().is_hidden());

    hook.failed(&"Windows refused the keyboard hook");
    show_until(&mut window, &wakeups, |window| !window.state().is_hidden());
    window.run_ok();

    window.get_by_label_contains("No trigger: Windows refused the keyboard hook");
}
