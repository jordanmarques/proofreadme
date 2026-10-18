//! The settings window, driven headless with `egui_kittest`, plugged into a real agent over a
//! simulated data folder, an in-memory keychain and a local provider.

mod support;

use std::fs;
use std::time::Instant;

use egui::{Key as EguiKey, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use proofreadme::Window;
use proofreadme_core::{Direction, Key, KeyEvent, MemoryKeychain, Modifier, Settings, TEST_SENTENCE};
use support::{API_KEY, LocalProvider, Reply, config, loaded, observed, show_until};

/// A window over this agent, Kairo installed, tall enough to show the whole form without
/// scrolling.
fn window<'a>(agent: proofreadme_core::Agent) -> Harness<'a, Window> {
    let mut window = Harness::builder().with_size(egui::vec2(760.0, 1600.0)).build_eframe(|creation| {
        kairo::install(&creation.egui_ctx);
        Window::new(agent)
    });
    window.run_ok();
    window
}

/// Replaces the text of the field with this label.
fn replace_text(window: &mut Harness<'_, Window>, label: &str, text: &str) {
    window.get_by_label(label).focus();
    window.run_ok();
    window.key_press_modifiers(Modifiers::COMMAND, EguiKey::A);
    window.run_ok();
    window.get_by_label(label).type_text(text);
    window.run_ok();
}

#[test]
fn at_first_launch_the_window_opens_on_the_settings_with_the_defaults() {
    let folder = tempfile::tempdir().unwrap();
    let (agent, _wakeups) = observed(loaded(folder.path(), &MemoryKeychain::default()));

    let window = window(agent);

    window.get_by_label("SAVE");
    assert_eq!(window.get_by_label("MODEL").value().as_deref(), Some("nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free"));
    assert_eq!(window.get_by_label("BASE URL").value().as_deref(), Some("https://openrouter.ai/api/v1"));
}

#[test]
fn saving_writes_the_config_and_puts_the_key_in_the_keychain_only() {
    let folder = tempfile::tempdir().unwrap();
    let keychain = MemoryKeychain::default();
    let (agent, _wakeups) = observed(loaded(folder.path(), &keychain));
    let mut window = window(agent);

    replace_text(&mut window, "MODEL", "other/model:free");
    replace_text(&mut window, "API KEY", API_KEY);
    window.get_by_label("SAVE").click();
    window.run_ok();

    window.get_by_label("SAVED");
    let text = fs::read_to_string(config(folder.path())).unwrap();
    assert!(text.contains("other/model:free"));
    assert!(!text.contains(API_KEY));
    assert_eq!(keychain.key().as_deref(), Some(API_KEY));
}

#[test]
fn a_prompt_without_the_placeholder_is_refused_with_a_visible_message_and_nothing_is_written() {
    let folder = tempfile::tempdir().unwrap();
    let keychain = MemoryKeychain::default();
    let (agent, _wakeups) = observed(loaded(folder.path(), &keychain));
    let mut window = window(agent);

    replace_text(&mut window, "PROOFREADING PROMPT", "Corrige ce texte.");
    window.get_by_label("SAVE").click();
    window.run_ok();

    window.get_by_label("REFUSED");
    window.get_by_label_contains("must contain {{TEXTE}}");
    assert!(!config(folder.path()).exists());
    assert_eq!(keychain.key(), None);
}

#[test]
fn the_test_button_shows_the_correction_of_the_local_provider_without_holding_the_window() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    let (reply, release) = Reply::correction("Je suis allé au marché hier, and it was a really nice day.").held();
    provider.reply(reply);
    let (agent, wakeups) = observed(loaded(folder.path(), &MemoryKeychain::default()));
    let mut window = window(agent);
    replace_text(&mut window, "BASE URL", &provider.base_url());
    replace_text(&mut window, "API KEY", API_KEY);

    window.get_by_label("TEST").click();
    window.run_ok();

    // The provider holds its answer: the window still draws, showing the Test as running.
    window.get_by_label("TESTING");
    assert!(provider.next_request().contains(TEST_SENTENCE));
    release.send(()).unwrap();
    show_until(&mut window, &wakeups, |window| window.query_by_label("CORRECTED").is_some());
    window.get_by_label_contains("really nice day");
    assert!(!config(folder.path()).exists(), "a Test saves nothing");
}

#[test]
fn a_failed_test_shows_its_cause() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    provider.reply(Reply::status(401));
    let (agent, wakeups) = observed(loaded(folder.path(), &MemoryKeychain::default()));
    let mut window = window(agent);
    replace_text(&mut window, "BASE URL", &provider.base_url());

    window.get_by_label("TEST").click();
    show_until(&mut window, &wakeups, |window| window.query_by_label("FAILED").is_some());

    window.get_by_label_contains("HTTP 401");
}

#[test]
fn a_trigger_without_a_key_opens_the_settings_and_sends_no_request() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    let keychain = MemoryKeychain::with_key(API_KEY);
    let settings = Settings { trigger: Modifier::Shift, provider: support::provider(&provider.base_url()), ..Settings::default() };
    loaded(folder.path(), &keychain).save_settings(settings.clone()).unwrap();
    let (agent, wakeups) = observed(loaded(folder.path(), &keychain));
    let mut window = window(agent);
    window.get_by_label("SETTINGS");
    // The key goes away without the settings screen: the window stays on the home screen.
    let mut keyless = settings;
    keyless.provider.api_key = String::new();
    window.state().agent().save_settings(keyless).unwrap();
    window.run_ok();
    assert!(window.query_by_label("SAVE").is_none());

    let now = Instant::now();
    for direction in [Direction::Down, Direction::Up, Direction::Down, Direction::Up] {
        window.state().agent().key_event(KeyEvent { key: Key::Modifier(Modifier::Shift), direction, at: now, injected: false });
    }
    show_until(&mut window, &wakeups, |window| window.query_by_label("SAVE").is_some());

    assert!(provider.received().is_empty());
}
