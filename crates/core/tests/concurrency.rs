//! What a trigger does when there is nothing to proofread, or a proofread is already running.

mod support;

use proofreadme_core::{Activity, ClipboardContent, Keystroke, Modifier, Step};
use support::{DELAY, Desktop, Keyboard, LocalProvider, Reply, agent, settings, wait_idle};

#[test]
fn a_trigger_with_nothing_selected_sends_nothing_and_touches_nothing() {
    let provider = LocalProvider::start();
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    let agent = agent(&desktop, settings(&provider));

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert_eq!(desktop.keystrokes(), vec![Keystroke::Copy]);
    assert!(provider.received().is_empty());
    assert!(desktop.writes().is_empty());
    assert_eq!(desktop.clipboard(), ClipboardContent::text("copied before".to_owned()));
    assert_eq!(agent.state().last_failure, None);
}

#[test]
fn a_blank_selection_sends_nothing_and_restores_the_clipboard() {
    let provider = LocalProvider::start();
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    desktop.select(" \n\t");
    let agent = agent(&desktop, settings(&provider));

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert!(provider.received().is_empty());
    assert!(desktop.pasted().is_empty());
    assert_eq!(desktop.clipboard(), ClipboardContent::text("copied before".to_owned()));
    assert_eq!(agent.state().last_failure, None);
}

#[test]
fn a_trigger_during_a_running_proofread_changes_nothing() {
    let provider = LocalProvider::start();
    let (reply, release) = Reply::correction("Hello world").held();
    provider.reply(reply);
    let desktop = Desktop::new();
    desktop.select("Helo world");
    let agent = agent(&desktop, settings(&provider));
    let mut keyboard = Keyboard::new();

    keyboard.double_press(&agent, Modifier::Ctrl);
    provider.next_request();
    assert_eq!(agent.state().activity, Activity::Proofreading(Step::Requesting));
    keyboard.wait(std::time::Duration::from_secs(1));
    keyboard.double_press(&agent, Modifier::Ctrl);
    release.send(()).unwrap();
    wait_idle(&agent);

    assert_eq!(desktop.keystrokes(), vec![Keystroke::Copy, Keystroke::Paste]);
    assert_eq!(desktop.pasted(), vec!["Hello world".to_owned()]);
    assert!(provider.received().is_empty(), "a second request reached the provider");
}

#[test]
fn the_agent_listens_again_after_a_proofread() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("First fixed"));
    provider.reply(Reply::correction("Second fixed"));
    let desktop = Desktop::new();
    let agent = agent(&desktop, settings(&provider));
    let mut keyboard = Keyboard::new();

    desktop.select("First fixd");
    keyboard.double_press(&agent, Modifier::Ctrl);
    assert!(agent.wait_until_idle(DELAY));
    keyboard.wait(std::time::Duration::from_secs(1));
    desktop.select("Second fixd");
    keyboard.double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert_eq!(desktop.pasted(), vec!["First fixed".to_owned(), "Second fixed".to_owned()]);
}
