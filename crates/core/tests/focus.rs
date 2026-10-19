//! The foreground application changed during the request (ADR 0002): the correction is not
//! pasted, it is left on the clipboard for the user, and a notification says so.

mod support;

use proofreadme_core::{ClipboardContent, Keystroke, Mark, Modifier, Notification};
use support::{Desktop, Keyboard, LocalProvider, Outside, Reply, settings, wait_idle, watched_agent};

#[test]
fn a_correction_for_an_application_left_meanwhile_stays_on_the_clipboard() {
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
    desktop.focus("browser");
    release.send(()).unwrap();
    wait_idle(&agent);

    assert_eq!(desktop.keystrokes(), vec![Keystroke::Copy], "nothing is pasted");
    assert_eq!(desktop.clipboard(), ClipboardContent::text("Hello world".to_owned()));
    assert_eq!(desktop.mark(), Some(Mark::Kept), "clipboard histories keep the correction");
    assert_eq!(outside.sent(), vec![Notification::LeftOnClipboard]);
    assert_eq!(agent.state().last_failure, None, "the correction is not lost: no failure");
    assert_eq!(outside.log(), "");
}
