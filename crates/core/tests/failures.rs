//! What can go wrong during a proofread: each failure says why in one notification, pastes
//! nothing and gives the clipboard back; the agent log keeps one line per failure, without the
//! text of the user.

mod support;

use proofreadme_core::{ClipboardContent, IconState, Keystroke, Modifier, Notification, OsError, ProofreadError};
use std::time::Duration;

use support::{API_KEY, Desktop, Keyboard, LocalProvider, MODEL, Outside, Reply, settings, wait_idle, watched_agent};

#[test]
fn an_error_status_is_notified_pastes_nothing_and_restores_the_clipboard() {
    let provider = LocalProvider::start();
    provider.reply(Reply::status(401));
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    desktop.select("Helo world");
    let outside = Outside::new();
    let agent = watched_agent(&desktop, settings(&provider), &outside);

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert_eq!(outside.sent(), vec![Notification::Failed(ProofreadError::Status(401))]);
    assert!(desktop.pasted().is_empty());
    assert_eq!(desktop.selection().as_deref(), Some("Helo world"));
    assert_eq!(desktop.clipboard(), ClipboardContent::text("copied before".to_owned()));
    assert_eq!(agent.state().last_failure, Some(ProofreadError::Status(401)));
}

/// Runs one proofread of "Helo world" over a clipboard holding "copied before", and checks what
/// every failure must leave: one notification with its cause, nothing pasted, the selection as it
/// was, the clipboard given back, the cause in the state.
fn fails_with(settings: proofreadme_core::Settings, cause: &dyn Fn(&ProofreadError) -> bool) {
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    desktop.select("Helo world");
    let outside = Outside::new();
    let agent = watched_agent(&desktop, settings, &outside);

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    let sent = outside.sent();
    assert_eq!(sent.len(), 1, "{sent:?}");
    let Notification::Failed(notified) = &sent[0] else { panic!("not a failure: {sent:?}") };
    assert!(cause(notified), "unexpected cause: {notified:?}");
    assert!(desktop.pasted().is_empty());
    assert_eq!(desktop.selection().as_deref(), Some("Helo world"));
    assert_eq!(desktop.clipboard(), ClipboardContent::text("copied before".to_owned()));
    assert_eq!(agent.state().last_failure.as_ref(), Some(notified));
    assert_eq!(outside.log().lines().count(), 1);
}

#[test]
fn an_unreachable_provider_is_notified() {
    let provider = LocalProvider::start();
    let mut settings = settings(&provider);
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    settings.provider.base_url = format!("http://{}/api/v1", closed.local_addr().unwrap());
    drop(closed);

    fails_with(settings, &|cause| matches!(cause, ProofreadError::Network(_)));
}

#[test]
fn a_provider_slower_than_the_timeout_is_given_up() {
    let provider = LocalProvider::start();
    let (reply, _release) = Reply::correction("Hello world").held();
    provider.reply(reply);
    let mut settings = settings(&provider);
    settings.timeout = Duration::from_millis(300);

    fails_with(settings, &|cause| *cause == ProofreadError::Timeout);
}

#[test]
fn an_empty_correction_is_notified() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("<texte>\n</texte>"));

    fails_with(settings(&provider), &|cause| *cause == ProofreadError::EmptyCorrection);
}

#[test]
fn each_failure_writes_one_line_to_the_log_without_the_text_of_the_user() {
    let provider = LocalProvider::start();
    provider.reply(Reply::status(500));
    provider.reply(Reply::correction("  "));
    let desktop = Desktop::new();
    let outside = Outside::new();
    let agent = watched_agent(&desktop, settings(&provider), &outside);
    let mut keyboard = Keyboard::new();

    desktop.select("Helo world, secret draft");
    keyboard.double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);
    keyboard.wait(Duration::from_secs(1));
    keyboard.double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    let log = outside.log();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 2, "{log}");
    assert!(lines[0].contains("provider error: HTTP 500"), "{log}");
    assert!(lines[1].contains("empty correction"), "{log}");
    for line in lines {
        assert!(line.contains(&format!("model={MODEL}")), "{line}");
        assert!(line.contains("duration="), "{line}");
    }
    assert!(!log.contains("secret draft"), "the selection reached the log");
    assert!(!log.contains(API_KEY), "the key reached the log");
}

#[test]
fn a_proofread_that_succeeds_writes_nothing_and_notifies_nothing() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("Hello world"));
    let desktop = Desktop::new();
    desktop.select("Helo world");
    let outside = Outside::new();
    let agent = watched_agent(&desktop, settings(&provider), &outside);

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert_eq!(desktop.pasted(), vec!["Hello world".to_owned()]);
    assert!(outside.sent().is_empty());
    assert_eq!(outside.log(), "");
}

#[test]
fn a_copy_the_os_refuses_ends_silently_like_an_empty_selection() {
    let provider = LocalProvider::start();
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    desktop.select("Helo world");
    // An application running elevated on Windows: the keystrokes of the agent do not reach it.
    desktop.refuse(Keystroke::Copy);
    let outside = Outside::new();
    let agent = watched_agent(&desktop, settings(&provider), &outside);

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert!(provider.received().is_empty(), "nothing was captured, nothing is sent");
    assert!(outside.sent().is_empty(), "nothing is notified");
    assert_eq!(outside.log(), "", "nothing is logged");
    assert_eq!(agent.state().last_failure, None);
    assert_eq!(agent.state().icon(), IconState::Idle);
    assert_eq!(desktop.selection().as_deref(), Some("Helo world"));
    assert_eq!(desktop.clipboard(), ClipboardContent::text("copied before".to_owned()));
}

#[test]
fn a_paste_the_os_refuses_is_a_failure() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("Hello world"));
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    desktop.select("Helo world");
    desktop.refuse(Keystroke::Paste);
    let outside = Outside::new();
    let agent = watched_agent(&desktop, settings(&provider), &outside);

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    let sent = outside.sent();
    assert!(matches!(sent.as_slice(), [Notification::Failed(ProofreadError::Os(OsError::Keystrokes(_)))]), "{sent:?}");
    assert_eq!(outside.log().lines().count(), 1);
    assert_eq!(desktop.selection().as_deref(), Some("Helo world"));
    assert_eq!(desktop.clipboard(), ClipboardContent::text("copied before".to_owned()));
}
