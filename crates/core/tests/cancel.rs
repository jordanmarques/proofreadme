//! Cancel from the menu of the session icon: the running proofread stops without pasting, gives
//! the clipboard back and tells nothing, and a late correction is dropped.

mod support;

use std::time::Duration;

use proofreadme_core::{Activity, ClipboardContent, Keystroke, Modifier};
use support::{DELAY, Desktop, Keyboard, LocalProvider, Outside, Reply, settings, wait_idle, watched_agent};

#[test]
fn cancel_during_a_slow_response_pastes_nothing_and_restores_the_clipboard() {
    let provider = LocalProvider::start();
    let (reply, release) = Reply::correction("Hello world").held();
    provider.reply(reply);
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    desktop.select("Helo world");
    let outside = Outside::new();
    let agent = watched_agent(&desktop, settings(&provider), &outside);

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    provider.next_request();
    agent.cancel();
    assert!(agent.wait_until_idle(DELAY), "the agent did not stop while the provider still holds its response");
    let after_cancel = (desktop.clipboard(), agent.state());
    release.send(()).unwrap();

    assert_eq!(after_cancel.0, ClipboardContent::text("copied before".to_owned()));
    assert_eq!(after_cancel.1.activity, Activity::Idle);
    assert_eq!(after_cancel.1.last_failure, None);
    assert!(outside.sent().is_empty(), "a cancel is not a failure");
    assert_eq!(outside.log(), "");
    assert_eq!(desktop.keystrokes(), vec![Keystroke::Copy]);
}

#[test]
fn a_new_proofread_runs_after_a_cancel_and_the_late_correction_is_dropped() {
    let provider = LocalProvider::start();
    let (late, release_late) = Reply::correction("Late correction").held();
    provider.reply(late);
    let (second, release_second) = Reply::correction("Second fixed").held();
    provider.reply(second);
    let desktop = Desktop::new();
    let agent = watched_agent(&desktop, settings(&provider), &Outside::new());
    let mut keyboard = Keyboard::new();

    desktop.select("First fixd");
    keyboard.double_press(&agent, Modifier::Ctrl);
    provider.next_request();
    agent.cancel();
    wait_idle(&agent);
    // The first request is still with the provider when the second proofread starts.
    keyboard.wait(Duration::from_secs(1));
    desktop.select("Second fixd");
    keyboard.double_press(&agent, Modifier::Ctrl);
    provider.next_request();
    release_late.send(()).unwrap();
    release_second.send(()).unwrap();
    wait_idle(&agent);

    assert_eq!(desktop.pasted(), vec!["Second fixed".to_owned()]);
}
