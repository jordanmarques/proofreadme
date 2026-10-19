//! The notification sink of the agent on the system: a notification of the OS (toast on Windows,
//! Notification Center on macOS) through `notify-rust`. No rule lives here.

use std::thread;

use proofreadme_core::{Notification, Notifier};

/// Shows the notifications of the agent as notifications of the OS.
#[derive(Debug)]
pub struct SystemNotifier {
    /// Windows: the AppUserModelID the toasts are sent under, once registered. Without it they are
    /// sent under that of PowerShell, and show "Windows PowerShell" as their source.
    app_id: Option<&'static str>,
}

impl SystemNotifier {
    /// The sink of the system. On macOS the notifications are sent in the name of the bundle of
    /// ProofReadMe; launched outside the bundle, they borrow the name of another application. On
    /// Windows the AppUserModelID of ProofReadMe is registered for the user (name and icon of its
    /// toasts, no Start menu shortcut needed).
    #[must_use]
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        if let Err(error) = notify_rust::set_application(crate::IDENTIFIER) {
            eprintln!("ProofReadMe: the notifications are not sent in the name of the application: {error}");
        }
        #[cfg(windows)]
        let app_id = match register_app_id() {
            Ok(()) => Some(crate::IDENTIFIER),
            Err(error) => {
                eprintln!("ProofReadMe: the notifications are not sent in the name of the application: {error}");
                None
            }
        };
        #[cfg(not(windows))]
        let app_id = None;
        Self { app_id }
    }
}

impl Default for SystemNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifier for SystemNotifier {
    fn notify(&self, notification: &Notification) {
        let message = notification.message();
        let app_id = self.app_id;
        // Shown on a thread of its own: the proofread never waits on the notification center.
        let spawned = thread::Builder::new().name("notification".to_owned()).spawn(move || {
            let mut toast = notify_rust::Notification::new();
            toast.summary(crate::NAME).body(&message);
            // `notify-rust` only takes an AppUserModelID on Windows; elsewhere there is none.
            #[cfg(windows)]
            if let Some(app_id) = app_id {
                toast.app_id(app_id);
            }
            #[cfg(not(windows))]
            let _ = app_id;
            if let Err(error) = toast.show() {
                eprintln!("ProofReadMe: a notification could not be shown: {error}");
            }
        });
        if let Err(error) = spawned {
            eprintln!("ProofReadMe: a notification could not be shown: {error}");
        }
    }
}

/// Registers the AppUserModelID of ProofReadMe for the current user, under
/// `HKCU\Software\Classes\AppUserModelId\<identifier>`: the display name and the icon Windows shows
/// on its toasts. Idempotent; the icon is written next to the config.
#[cfg(windows)]
fn register_app_id() -> Result<(), Box<dyn std::error::Error>> {
    let key = windows_registry::CURRENT_USER.create(format!(r"Software\Classes\AppUserModelId\{}", crate::IDENTIFIER))?;
    key.set_string("DisplayName", crate::NAME)?;
    let folder = crate::data_folder()?;
    std::fs::create_dir_all(&folder)?;
    let icon = folder.join(format!("{}.ico", crate::NAME));
    std::fs::write(&icon, proofreadme_icon::ico())?;
    key.set_string("IconUri", icon.to_string_lossy())?;
    Ok(())
}
