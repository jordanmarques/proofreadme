//! The permission panel: shown above either screen while the keyboard hook lacks the permission
//! of the OS (Accessibility on macOS), as the agent reports it.

use egui::Ui;
use kairo::{Button, Panel, StatusLight, Tone, fonts, tokens};
use proofreadme_core::Permission;

use crate::Action;

/// Draws the panel for this permission, if it is not granted, and returns what the user asked
/// for. `can_open_pane` tells whether the window knows how to open the pane of System Settings.
pub(crate) fn show(ui: &mut Ui, permission: Permission, can_open_pane: bool) -> Option<Action> {
    let mut action = None;
    match permission {
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

fn body(ui: &mut Ui, text: &str) {
    ui.label(fonts::body(text, tokens::TEXT_MD, tokens::FG_1));
}
