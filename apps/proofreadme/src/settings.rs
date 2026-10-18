//! The settings screen: the form of the settings being edited, the outcome of the Test of the
//! provider and the refusal of the last save, as the agent reports them.

use std::time::Duration;

use egui::{DragValue, TextEdit, Ui};
use kairo::fonts;
use kairo::{Button, Panel, StatusLight, Switch, Tone, tokens};
use proofreadme_core::{Modifier, ProviderTest, Settings, SettingsError, State};

use crate::{Action, SessionNotice};

/// The trigger keys offered, with their labels.
const TRIGGER_KEYS: [(Modifier, &str); 4] =
    [(Modifier::Ctrl, "Ctrl"), (Modifier::Alt, "Alt / Option"), (Modifier::Shift, "Shift"), (Modifier::Cmd, "Cmd")];

/// The longest timeout the form offers, in seconds.
const MAX_TIMEOUT_SECONDS: u64 = 600;

/// Draws the form over `draft`, the settings being edited, and returns what the user asked for.
pub(crate) fn show(ui: &mut Ui, draft: &mut Settings, state: &State, saved: bool, session: &SessionNotice<'_>) -> Option<Action> {
    let mut action = None;
    Panel::new().title("Provider").jp("接続").code("PRV-01").tone(Tone::Accent).show(ui, |ui| {
        field(ui, "Base URL", &mut draft.provider.base_url, false);
        field(ui, "API key", &mut draft.provider.api_key, true);
        field(ui, "Model", &mut draft.provider.model, false);
        if ui.add(Button::secondary("Test").jp("試験")).clicked() {
            action = Some(Action::Test);
        }
        show_test(ui, state.provider_test.as_ref());
    });
    Panel::new().title("Proofreading prompt").jp("指示").code("PRM-02").tone(Tone::Neutral).show(ui, |ui| {
        let label = ui.label(fonts::label("Proofreading prompt", tokens::TEXT_XS, tokens::FG_2));
        ui.add(TextEdit::multiline(&mut draft.prompt).desired_rows(12).desired_width(f32::INFINITY)).labelled_by(label.id);
        note(ui, "The selection takes the place of {{TEXTE}}.");
    });
    Panel::new().title("Trigger").jp("起動").code("TRG-03").tone(Tone::Neutral).show(ui, |ui| {
        ui.label(fonts::label("Trigger key", tokens::TEXT_XS, tokens::FG_2));
        ui.horizontal(|ui| {
            for (modifier, name) in TRIGGER_KEYS {
                let button = if draft.trigger == modifier { Button::primary(name) } else { Button::secondary(name) };
                if ui.add(button).clicked() {
                    draft.trigger = modifier;
                }
            }
        });
        note(ui, "Press it twice, quickly, with text selected.");
        let label = ui.label(fonts::label("Timeout (seconds)", tokens::TEXT_XS, tokens::FG_2));
        let mut seconds = draft.timeout.as_secs();
        if ui.add(DragValue::new(&mut seconds).range(1..=MAX_TIMEOUT_SECONDS)).labelled_by(label.id).changed() {
            draft.timeout = Duration::from_secs(seconds);
        }
        ui.add(Switch::new(&mut draft.reasoning).label("Reasoning").jp("推論"));
        ui.add(Switch::new(&mut draft.launch_at_login).label("Launch at login").jp("自動起動"));
        if let Some(error) = session.registration_error {
            ui.add(StatusLight::warn().label("Login item").jp("登録"));
            body(ui, &sentence(error));
        }
        if session.no_icon {
            note(ui, "No session icon: closing the window quits ProofReadMe.");
        }
    });
    ui.horizontal(|ui| {
        if ui.add(Button::primary("Save").jp("保存")).clicked() {
            action = Some(Action::Save);
        }
        if ui.add(Button::ghost("Back").jp("戻る")).clicked() {
            action = Some(Action::Back);
        }
    });
    if let Some(error) = &state.settings_error {
        let light = if matches!(error, SettingsError::KeyUnreadable(_)) {
            StatusLight::warn().label("Keychain").jp("鍵")
        } else {
            StatusLight::warn().label("Refused").jp("拒否")
        };
        ui.add(light);
        body(ui, &sentence(&error.to_string()));
    } else if saved {
        ui.add(StatusLight::ok().label("Saved").jp("保存済"));
    }
    action
}

/// The outcome of the Test of the provider.
fn show_test(ui: &mut Ui, test: Option<&ProviderTest>) {
    match test {
        None => {}
        Some(ProviderTest::Running) => {
            ui.add(StatusLight::info().blink(true).label("Testing").jp("試験中"));
        }
        Some(ProviderTest::Corrected(correction)) => {
            ui.add(StatusLight::ok().label("Corrected").jp("校正済"));
            ui.label(fonts::mono(correction.clone(), tokens::TEXT_MD, tokens::FG_1));
        }
        Some(ProviderTest::Failed(cause)) => {
            ui.add(StatusLight::alert().label("Failed").jp("失敗"));
            body(ui, &sentence(&cause.to_string()));
        }
    }
}

/// A text field with its label above it.
fn field(ui: &mut Ui, name: &str, text: &mut String, password: bool) {
    let label = ui.label(fonts::label(name, tokens::TEXT_XS, tokens::FG_2));
    ui.add(TextEdit::singleline(text).password(password).desired_width(f32::INFINITY)).labelled_by(label.id);
}

/// An error message of the core as a sentence: capitalised, with a full stop.
fn sentence(message: &str) -> String {
    let mut characters = message.chars();
    characters.next().map_or_else(String::new, |first| format!("{}{}.", first.to_uppercase(), characters.as_str()))
}

fn body(ui: &mut Ui, text: &str) {
    ui.label(fonts::body(text, tokens::TEXT_MD, tokens::FG_1));
}

fn note(ui: &mut Ui, text: &str) {
    ui.label(fonts::body(text, tokens::TEXT_SM, tokens::FG_3));
}
