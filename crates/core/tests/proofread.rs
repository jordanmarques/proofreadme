//! A full proofread, driven headless: the trigger, the copy of the selection, the request to the
//! provider, the paste of the correction and the restoration of the clipboard (ADR 0002).

mod support;

use proofreadme_core::{Activity, ClipboardContent, Keystroke, Mark, Modifier};
use serde_json::json;
use support::{API_KEY, Desktop, Keyboard, LocalProvider, MODEL, Reply, agent, image, settings, wait_idle};

#[test]
fn a_trigger_replaces_the_selection_by_its_correction_and_restores_the_clipboard() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("Il a mangé une pomme."));
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    desktop.select("Il a manger une pome.");
    let agent = agent(&desktop, settings(&provider));

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    let request = provider.next_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.url, "/api/v1/chat/completions");
    assert_eq!(request.authorization.as_deref(), Some(format!("Bearer {API_KEY}").as_str()));
    assert!(request.content_type.unwrap_or_default().starts_with("application/json"));
    assert_eq!(
        request.body,
        json!({
            "model": MODEL,
            "messages": [{ "role": "user", "content": "Corrige :\n<texte>\nIl a manger une pome.\n</texte>" }],
            "reasoning": { "enabled": true },
        })
    );
    assert_eq!(desktop.keystrokes(), vec![Keystroke::Copy, Keystroke::Paste]);
    assert_eq!(desktop.pasted(), vec!["Il a mangé une pomme.".to_owned()]);
    assert_eq!(desktop.selection().as_deref(), Some("Il a mangé une pomme."));
    assert_eq!(desktop.clipboard(), ClipboardContent::text("copied before".to_owned()));
    assert_eq!(agent.state().activity, Activity::Idle);
    assert_eq!(agent.state().last_failure, None);
}

#[test]
fn every_write_of_the_agent_is_transient_and_a_copied_image_is_restored() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("Hello world"));
    let desktop = Desktop::new();
    let copied = ClipboardContent { text: None, image: Some(image()) };
    desktop.user_copies(copied.clone());
    desktop.select("Helo world");
    let agent = agent(&desktop, settings(&provider));

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert_eq!(desktop.pasted(), vec!["Hello world".to_owned()]);
    let writes = desktop.writes();
    assert_eq!(writes, vec![(ClipboardContent::text("Hello world".to_owned()), Mark::Transient), (copied.clone(), Mark::Transient)]);
    assert_eq!(desktop.clipboard(), copied);
}

#[test]
fn an_empty_clipboard_is_left_empty() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("Hello world"));
    let desktop = Desktop::new();
    desktop.select("Helo world");
    let agent = agent(&desktop, settings(&provider));

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert_eq!(desktop.pasted(), vec!["Hello world".to_owned()]);
    assert_eq!(desktop.clipboard(), ClipboardContent::default());
}
