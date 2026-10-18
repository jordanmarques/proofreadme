//! The window of ProofReadMe, driven headless with `egui_kittest`.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use proofreadme::Window;

#[test]
fn the_window_is_titled_proofreadme() {
    let options = proofreadme::window_options();

    assert_eq!(options.viewport.title.as_deref(), Some("ProofReadMe"));
}

#[test]
fn the_window_opens_dressed_with_kairo_and_shows_the_name_of_the_agent() {
    let mut window = Harness::builder().build_eframe(|creation| {
        kairo::install(&creation.egui_ctx);
        Window::new()
    });

    window.run_ok();

    window.get_by_label_contains("PROOFREADME");
}
