//! The permission the keyboard hook needs (Accessibility or Input Monitoring on macOS), as the
//! platform reports it to the agent through its hook, and what the agent shows of it.

mod support;

use std::thread;

use proofreadme_core::{Direction, IconState, Key, KeyEvent, Modifier, NoTrigger, Permission};
use support::{Desktop, LocalProvider, Outside, Reply, agent, settings, wait_idle, watched_agent};

#[test]
fn an_agent_starts_with_the_permission_granted() {
    let provider = LocalProvider::start();
    let agent = agent(&Desktop::new(), settings(&provider));

    assert_eq!(agent.state().permission, Permission::Granted);
    assert_eq!(agent.state().icon(), IconState::Idle);
}

#[test]
fn a_missing_permission_shows_the_alert_icon_until_it_is_granted() {
    let provider = LocalProvider::start();
    let agent = agent(&Desktop::new(), settings(&provider));
    let hook = agent.hook();

    hook.permission(Permission::Missing);
    let missing = agent.state();
    hook.permission(Permission::Granted);

    assert_eq!(missing.permission, Permission::Missing);
    assert_eq!(missing.icon(), IconState::Alert);
    assert_eq!(agent.state().icon(), IconState::Idle, "granted, the alert goes away");
}

#[test]
fn a_permission_granted_that_needs_a_relaunch_keeps_the_alert_icon() {
    let provider = LocalProvider::start();
    let agent = agent(&Desktop::new(), settings(&provider));

    agent.hook().permission(Permission::RelaunchNeeded);

    assert_eq!(agent.state().permission, Permission::RelaunchNeeded);
    assert_eq!(agent.state().icon(), IconState::Alert);
}

#[test]
fn losing_the_permission_asks_for_the_window_once_per_loss() {
    let provider = LocalProvider::start();
    let agent = agent(&Desktop::new(), settings(&provider));
    let hook = agent.hook();
    assert_eq!(agent.state().window_requests, 0);

    hook.permission(Permission::Missing);
    hook.permission(Permission::Missing);
    let after_the_loss = agent.state().window_requests;
    hook.permission(Permission::Granted);
    let once_granted = agent.state().window_requests;
    hook.permission(Permission::RelaunchNeeded);

    assert_eq!(after_the_loss, 1, "a loss reported twice is one loss");
    assert_eq!(once_granted, 1, "a granted permission needs nobody");
    assert_eq!(agent.state().window_requests, 2);
}

#[test]
fn a_hook_that_cannot_listen_shows_the_alert_icon_asks_for_the_window_and_is_logged() {
    let provider = LocalProvider::start();
    let outside = Outside::new();
    let agent = watched_agent(&Desktop::new(), settings(&provider), &outside);

    agent.hook().failed(&"Windows refused the keyboard hook");

    let state = agent.state();
    assert_eq!(state.no_trigger, Some(NoTrigger("Windows refused the keyboard hook".to_owned())));
    assert_eq!(state.icon(), IconState::Alert, "the agent is deaf to the trigger");
    assert_eq!(state.window_requests, 1, "the window says why");
    assert!(outside.log().contains("no trigger: Windows refused the keyboard hook"), "{}", outside.log());
}

#[test]
fn a_change_of_the_permission_wakes_the_observer() {
    let provider = LocalProvider::start();
    let (sender, wakeups) = std::sync::mpsc::channel();
    let agent = agent(&Desktop::new(), settings(&provider)).on_change(move || {
        let _ = sender.send(());
    });

    agent.hook().permission(Permission::Missing);

    assert!(wakeups.try_recv().is_ok());
}

#[test]
fn the_hook_feeds_the_trigger_from_its_own_thread() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("Hello world"));
    let desktop = Desktop::new();
    desktop.select("Helo world");
    let agent = agent(&desktop, settings(&provider));
    let hook = agent.hook();

    thread::spawn(move || {
        let ctrl = Key::Modifier(Modifier::Ctrl);
        for direction in [Direction::Down, Direction::Up, Direction::Down, Direction::Up] {
            hook.key_event(KeyEvent { key: ctrl, direction, at: std::time::Instant::now(), injected: false });
        }
    })
    .join()
    .unwrap();
    provider.next_request();
    wait_idle(&agent);

    assert_eq!(desktop.pasted(), ["Hello world"]);
}
