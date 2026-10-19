//! The trigger (ADR 0003): two presses of the configured modifier within 300 ms, no other key down
//! in between, firing on the second release. A trigger shows as the copy the agent synthesizes.

mod support;

use std::time::Duration;

use proofreadme_core::{Agent, Direction, Key, KeyEvent, Keystroke, Modifier};
use support::{Desktop, Keyboard, LocalProvider, Reply, agent, settings, wait_idle};

/// An agent whose trigger is Ctrl, over a desktop with a selection.
fn listening() -> (LocalProvider, Desktop, Agent) {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("Fixed text"));
    let desktop = Desktop::new();
    desktop.select("Fixd text");
    let agent = agent(&desktop, settings(&provider));
    (provider, desktop, agent)
}

/// Whether the key events so far made a trigger: once no proofread runs, a trigger has left its
/// copy behind.
fn triggered(desktop: &Desktop, agent: &Agent) -> bool {
    wait_idle(agent);
    desktop.keystrokes().contains(&Keystroke::Copy)
}

const CTRL: Key = Key::Modifier(Modifier::Ctrl);

#[test]
fn a_double_press_fires_on_the_second_release_only() {
    let (_provider, desktop, agent) = listening();
    let mut keyboard = Keyboard::new();

    keyboard.tap(&agent, CTRL);
    keyboard.down(&agent, CTRL);
    assert!(!triggered(&desktop, &agent), "fired before the second release");

    keyboard.wait(Duration::from_millis(40));
    keyboard.up(&agent, CTRL);
    assert!(triggered(&desktop, &agent));
}

#[test]
fn two_presses_too_far_apart_do_not_fire() {
    let (_provider, desktop, agent) = listening();
    let mut keyboard = Keyboard::new();

    keyboard.tap(&agent, CTRL);
    keyboard.wait(Duration::from_millis(300));
    keyboard.tap(&agent, CTRL);

    assert!(!triggered(&desktop, &agent));
}

#[test]
fn a_slow_press_followed_by_a_quick_double_press_fires() {
    let (_provider, desktop, agent) = listening();
    let mut keyboard = Keyboard::new();

    keyboard.tap(&agent, CTRL);
    keyboard.wait(Duration::from_secs(1));
    keyboard.tap(&agent, CTRL);
    keyboard.tap(&agent, CTRL);

    assert!(triggered(&desktop, &agent));
}

#[test]
fn another_key_in_between_does_not_fire() {
    let (_provider, desktop, agent) = listening();
    let mut keyboard = Keyboard::new();

    keyboard.down(&agent, CTRL);
    keyboard.tap(&agent, Key::Other(0x43));
    keyboard.up(&agent, CTRL);
    keyboard.tap(&agent, CTRL);

    assert!(!triggered(&desktop, &agent));
}

#[test]
fn a_double_press_of_another_modifier_does_not_fire() {
    let (_provider, desktop, agent) = listening();

    Keyboard::new().double_press(&agent, Modifier::Shift);

    assert!(!triggered(&desktop, &agent));
}

#[test]
fn the_configured_modifier_is_the_trigger() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("Fixed text"));
    let desktop = Desktop::new();
    desktop.select("Fixd text");
    let agent = agent(&desktop, proofreadme_core::Settings { trigger: Modifier::Alt, ..settings(&provider) });
    let mut keyboard = Keyboard::new();

    keyboard.double_press(&agent, Modifier::Ctrl);
    assert!(!triggered(&desktop, &agent));

    keyboard.wait(Duration::from_secs(1));
    keyboard.double_press(&agent, Modifier::Alt);
    assert!(triggered(&desktop, &agent));
}

#[test]
fn a_held_modifier_repeating_its_press_is_one_press() {
    let (_provider, desktop, agent) = listening();
    let mut keyboard = Keyboard::new();

    keyboard.down(&agent, CTRL);
    keyboard.wait(Duration::from_millis(30));
    keyboard.down(&agent, CTRL);
    keyboard.wait(Duration::from_millis(30));
    keyboard.up(&agent, CTRL);

    assert!(!triggered(&desktop, &agent));
}

#[test]
fn injected_key_events_never_count() {
    let (_provider, desktop, agent) = listening();
    let start = std::time::Instant::now();
    let event = |direction, ms| KeyEvent { key: CTRL, direction, at: start + Duration::from_millis(ms), injected: true };

    for (direction, ms) in [(Direction::Down, 0), (Direction::Up, 20), (Direction::Down, 40), (Direction::Up, 60)] {
        agent.key_event(event(direction, ms));
    }

    assert!(!triggered(&desktop, &agent));
}

#[test]
fn a_keyboard_hook_on_its_own_thread_feeds_the_agent_through_a_clone() {
    let (_provider, desktop, agent) = listening();
    let hook = agent.clone();

    std::thread::spawn(move || Keyboard::new().double_press(&hook, Modifier::Ctrl)).join().unwrap();

    assert!(triggered(&desktop, &agent));
}
