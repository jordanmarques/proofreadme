//! The request sent to the provider and the cleaning of what comes back, seen from what lands in
//! the application.

mod support;

use proofreadme_core::{ClipboardContent, Modifier, Settings};
use serde_json::json;
use support::{Desktop, Keyboard, LocalProvider, MODEL, Reply, agent, settings, wait_idle};

/// Proofreads the selection with the given settings and the given text returned by the provider;
/// returns what was pasted and the body of the request.
fn proofread(selection: &str, returned: &str, adjust: impl FnOnce(Settings) -> Settings) -> (Vec<String>, serde_json::Value) {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction(returned));
    let desktop = Desktop::new();
    desktop.select(selection);
    let agent = agent(&desktop, adjust(settings(&provider)));
    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);
    (desktop.pasted(), provider.next_request().body)
}

#[test]
fn without_reasoning_the_request_carries_no_reasoning_field() {
    let (_, body) = proofread("Helo", "Hello", |settings| Settings { reasoning: false, ..settings });

    assert_eq!(body, json!({ "model": MODEL, "messages": [{ "role": "user", "content": "Corrige :\n<texte>\nHelo\n</texte>" }] }));
}

#[test]
fn with_reasoning_the_request_enables_it() {
    let (_, body) = proofread("Helo", "Hello", |settings| Settings { reasoning: true, ..settings });

    assert_eq!(body["reasoning"], json!({ "enabled": true }));
}

#[test]
fn the_selection_takes_the_place_of_every_placeholder_and_nothing_else() {
    let (_, body) = proofread("a {{nom}} b", "a {{nom}} c", |settings| Settings { prompt: "{{TEXTE}} | {{TEXTE}}".to_owned(), ..settings });

    assert_eq!(body["messages"][0]["content"], "a {{nom}} b | a {{nom}} b");
}

#[test]
fn a_correction_wrapped_in_texte_tags_is_unwrapped() {
    let (pasted, _) = proofread("Bonjour a tous", "<texte>\nBonjour à tous\n</texte>", |settings| settings);

    assert_eq!(pasted, vec!["Bonjour à tous".to_owned()]);
}

#[test]
fn tags_inside_the_correction_are_kept() {
    let (pasted, _) = proofread("Use <texte> tag", "Use the <texte> tag", |settings| settings);

    assert_eq!(pasted, vec!["Use the <texte> tag".to_owned()]);
}

#[test]
fn code_fences_of_the_selection_are_kept() {
    let selection = "```rust\nlet x = 1;\n```\nThis are code.";
    let returned = "```rust\nlet x = 1;\n```\nThis is code.";

    let (pasted, _) = proofread(selection, returned, |settings| settings);

    assert_eq!(pasted, vec![returned.to_owned()]);
}

#[test]
fn the_surrounding_whitespace_of_the_selection_is_preserved() {
    let (pasted, _) = proofread("\n  Helo world. \n\n", "Hello world.\n", |settings| settings);

    assert_eq!(pasted, vec!["\n  Hello world. \n\n".to_owned()]);
}

#[test]
fn a_correction_identical_to_the_selection_is_not_pasted_and_the_clipboard_is_restored() {
    let provider = LocalProvider::start();
    provider.reply(Reply::correction("<texte>All good.</texte>"));
    let desktop = Desktop::new();
    desktop.user_copies(ClipboardContent::text("copied before".to_owned()));
    desktop.select(" All good.");
    let agent = agent(&desktop, settings(&provider));

    Keyboard::new().double_press(&agent, Modifier::Ctrl);
    wait_idle(&agent);

    assert_eq!(provider.received().len(), 1);
    assert!(desktop.pasted().is_empty());
    assert_eq!(desktop.selection().as_deref(), Some(" All good."));
    assert_eq!(desktop.clipboard(), ClipboardContent::text("copied before".to_owned()));
    assert_eq!(agent.state().last_failure, None);
}
