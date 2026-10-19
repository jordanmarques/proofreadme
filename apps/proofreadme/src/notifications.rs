//! The notification sink of the agent on the system: a notification of the OS (toast on Windows,
//! Notification Center on macOS) through `notify-rust`. No rule lives here.

use std::thread;

use proofreadme_core::{Notification, Notifier};

/// Shows the notifications of the agent as notifications of the OS.
#[derive(Debug)]
pub struct SystemNotifier;

impl SystemNotifier {
    /// The sink of the system. On macOS the notifications are sent in the name of the bundle of
    /// ProofReadMe; launched outside the bundle, they borrow the name of another application.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        if let Err(error) = notify_rust::set_application(crate::IDENTIFIER) {
            eprintln!("ProofReadMe: the notifications are not sent in the name of the application: {error}");
        }
        Self
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
        // Shown on a thread of its own: the proofread never waits on the notification center.
        let spawned = thread::Builder::new().name("notification".to_owned()).spawn(move || {
            if let Err(error) = notify_rust::Notification::new().summary(crate::NAME).body(&message).show() {
                eprintln!("ProofReadMe: a notification could not be shown: {error}");
            }
        });
        if let Err(error) = spawned {
            eprintln!("ProofReadMe: a notification could not be shown: {error}");
        }
    }
}
