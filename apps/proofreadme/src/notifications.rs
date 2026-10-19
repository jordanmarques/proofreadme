//! The notification sink of the agent on the system: a notification of the OS (toast on Windows,
//! Notification Center on macOS) through `notify-rust`. No rule lives here; what fails is written
//! to the agent log.

use std::thread;

use proofreadme_core::{AgentLog, Notification, Notifier};

/// Shows the notifications of the agent as notifications of the OS.
#[derive(Debug)]
pub struct SystemNotifier {
    /// Windows: the AppUserModelID the toasts are sent under, once registered. Without it they are
    /// sent under that of PowerShell, and show "Windows PowerShell" as their source.
    app_id: Option<&'static str>,
    /// Where a notification that cannot be shown is told.
    log: AgentLog,
}

/// Why the notifications are not sent in the name of the application.
#[cfg(windows)]
#[derive(Debug, thiserror::Error)]
enum AppIdError {
    /// The registry refused the AppUserModelID.
    #[error("the registry refused the AppUserModelID: {0}")]
    Registry(String),
    /// The OS gives no data folder to keep the icon of the toasts in.
    #[error(transparent)]
    NoDataFolder(#[from] crate::LaunchError),
    /// The icon of the toasts could not be written.
    #[error("the icon of the notifications could not be written: {0}")]
    Icon(#[from] std::io::Error),
}

impl SystemNotifier {
    /// The sink of the system, which tells its failures in `log`. On macOS the notifications are
    /// sent in the name of the bundle of ProofReadMe; launched outside the bundle, they borrow the
    /// name of another application. On Windows the AppUserModelID of ProofReadMe is registered for
    /// the user (name and icon of its toasts, no Start menu shortcut needed).
    #[must_use]
    pub fn new(log: AgentLog) -> Self {
        #[cfg(target_os = "macos")]
        if let Err(error) = notify_rust::set_application(crate::IDENTIFIER) {
            log.note(&format!("the notifications are not sent in the name of the application: {error}"));
        }
        #[cfg(windows)]
        let app_id = match register_app_id() {
            Ok(()) => Some(crate::IDENTIFIER),
            Err(error) => {
                log.note(&format!("the notifications are not sent in the name of the application: {error}"));
                None
            }
        };
        #[cfg(not(windows))]
        let app_id = None;
        Self { app_id, log }
    }
}

impl Notifier for SystemNotifier {
    fn notify(&self, notification: &Notification) {
        let message = notification.message();
        let app_id = self.app_id;
        let log = self.log.clone();
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
                log.note(&format!("a notification could not be shown: {error}"));
            }
        });
        if let Err(error) = spawned {
            self.log.note(&format!("a notification could not be shown: {error}"));
        }
    }
}

/// Registers the AppUserModelID of ProofReadMe for the current user, under
/// `HKCU\Software\Classes\AppUserModelId\<identifier>`: the display name and the icon Windows shows
/// on its toasts. Idempotent; the icon is written next to the config.
#[cfg(windows)]
fn register_app_id() -> Result<(), AppIdError> {
    let refused = |error: windows_registry::Result<()>| error.map_err(|error| AppIdError::Registry(error.to_string()));
    let key = windows_registry::CURRENT_USER
        .create(format!(r"Software\Classes\AppUserModelId\{}", crate::IDENTIFIER))
        .map_err(|error| AppIdError::Registry(error.to_string()))?;
    refused(key.set_string("DisplayName", crate::NAME))?;
    let folder = crate::data_folder()?;
    std::fs::create_dir_all(&folder)?;
    let icon = folder.join(format!("{}.ico", crate::NAME));
    std::fs::write(&icon, proofreadme_icon::ico())?;
    refused(key.set_string("IconUri", icon.to_string_lossy()))?;
    Ok(())
}
