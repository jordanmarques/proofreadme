//! The permission panel: shown above the settings while the agent is deaf to the trigger, as it
//! reports it: the keyboard hook lacks the permission of the OS (Accessibility on macOS), or cannot
//! listen at all.

use egui::Ui;
use kairo::{Button, Panel, StatusLight, Tone};
use proofreadme_core::{Permission, State};

use crate::Action;
use crate::settings::{body, sentence};

/// Draws the panel for this state, if the agent is deaf, and returns what the user asked for.
/// `can_open_pane` tells whether the window knows how to open the pane of System Settings.
pub(crate) fn show(ui: &mut Ui, state: &State, can_open_pane: bool) -> Option<Action> {
    let mut action = None;
    if let Some(no_trigger) = &state.no_trigger {
        Panel::new().title("Trigger").jp("起動").code("HOOK-00").tone(Tone::Alert).show(ui, |ui| {
            ui.add(StatusLight::alert().label("Trigger off").jp("停止"));
            body(ui, &sentence(&format!("{no_trigger}. Relaunch ProofReadMe to try again")));
        });
    }
    match state.permission {
        Permission::Granted => {}
        Permission::Missing => {
            Panel::new().title("Permission").jp("許可").code("ACC-00").tone(Tone::Alert).show(ui, |ui| {
                ui.add(StatusLight::alert().label("Trigger off").jp("停止"));
                body(
                    ui,
                    "ProofReadMe needs the Accessibility permission to see the trigger and to copy and paste the selection. \
                     Turn ProofReadMe on in Privacy & Security > Accessibility.",
                );
                if can_open_pane && ui.add(Button::primary("Open System Settings").jp("設定")).clicked() {
                    action = Some(Action::OpenPermissionPane);
                }
            });
        }
        Permission::RelaunchNeeded => {
            Panel::new().title("Permission").jp("許可").code("ACC-00").tone(Tone::Warn).show(ui, |ui| {
                ui.add(StatusLight::warn().label("Relaunch").jp("再起動"));
                body(ui, "The Accessibility permission is granted. Relaunch ProofReadMe for the trigger to work.");
            });
        }
    }
    action
}
