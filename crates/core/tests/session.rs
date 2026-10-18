//! What the session icon shows of the agent: its look (idle, running, alert) and whether its menu
//! offers to cancel, both read from the state of the agent.

mod support;

use proofreadme_core::{IconState, Modifier};
use support::{Desktop, Keyboard, LocalProvider, Reply, agent, settings, wait_idle};

#[test]
fn a_listening_agent_shows_the_idle_icon_and_offers_no_cancel() {
    let provider = LocalProvider::start();
    let agent = agent(&Desktop::new(), settings(&provider));

    let state = agent.state();

    assert_eq!(state.icon(), IconState::Idle);
    assert!(!state.can_cancel());
}

#[test]
fn a_running_proofread_shows_the_running_icon_and_offers_to_cancel() {
    let provider = LocalProvider::start();
    let (reply, release) = Reply::correction("Hello world").held();
    provider.reply(reply);
    let desktop = Desktop::new();
    desktop.select("Helo world");
    let agent = agent(&desktop, settings(&provider));

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    provider.next_request();
    let during = agent.state();
    release.send(()).unwrap();
    wait_idle(&agent);

    assert_eq!(during.icon(), IconState::Running);
    assert!(during.can_cancel());
    assert_eq!(agent.state().icon(), IconState::Idle, "back to idle once the correction is pasted");
    assert!(!agent.state().can_cancel());
}

#[test]
fn a_failed_proofread_shows_the_alert_icon_until_the_next_one_starts() {
    let provider = LocalProvider::start();
    provider.reply(Reply::status(500));
    let (reply, release) = Reply::correction("Hello world").held();
    provider.reply(reply);
    let desktop = Desktop::new();
    desktop.select("Helo world");
    let agent = agent(&desktop, settings(&provider));
    let mut keyboard = Keyboard::new();

    keyboard.double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);
    let after_the_failure = agent.state();
    keyboard.wait(std::time::Duration::from_secs(1));
    keyboard.double_press(&agent, Modifier::Ctrl);
    provider.next_request();
    provider.next_request();
    let next = agent.state();
    release.send(()).unwrap();
    wait_idle(&agent);

    assert_eq!(after_the_failure.icon(), IconState::Alert);
    assert!(!after_the_failure.can_cancel(), "nothing runs any more");
    assert_eq!(next.icon(), IconState::Running, "a new proofread clears the alert");
}

#[test]
fn cancel_without_a_running_proofread_changes_nothing() {
    let provider = LocalProvider::start();
    let desktop = Desktop::new();
    let agent = agent(&desktop, settings(&provider));
    let before = agent.state();

    agent.cancel();

    assert_eq!(agent.state(), before);
    assert!(provider.received().is_empty());
}
