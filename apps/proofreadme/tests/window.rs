//! The window of ProofReadMe, driven headless with `egui_kittest`.

mod support;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use proofreadme::Window;
use proofreadme_core::{MemoryKeychain, Settings};

#[test]
fn the_window_is_titled_proofreadme() {
    let options = proofreadme::window_options();

    assert_eq!(options.viewport.title.as_deref(), Some("ProofReadMe"));
}

#[test]
fn a_configured_agent_opens_dressed_with_kairo_on_the_name_of_the_agent() {
    let folder = tempfile::tempdir().unwrap();
    let keychain = MemoryKeychain::with_key(support::API_KEY);
    support::loaded(folder.path(), &keychain)
        .save_settings(Settings { provider: support::provider("http://127.0.0.1:9/api/v1"), ..Settings::default() })
        .unwrap();
    let (agent, _wakeups) = support::observed(support::loaded(folder.path(), &keychain));
    let mut window = Harness::builder().build_eframe(|creation| {
        kairo::install(&creation.egui_ctx);
        Window::new(agent)
    });

    window.run_ok();

    window.get_by_label_contains("PROOFREADME");
    window.get_by_label("SETTINGS");
    assert!(window.query_by_label("SAVE").is_none(), "the settings are not shown");
}
