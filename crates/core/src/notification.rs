//! The notifications of the agent: what it tells the user outside its window, through the
//! notification sink of the OS. None carries the selection, the correction or the API key.

use crate::error::ProofreadError;

/// What the agent tells the user at the end of a proofread that did not replace the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notification {
    /// The proofread failed, for this cause: the selection is unchanged.
    Failed(ProofreadError),
    /// The foreground application changed during the request: the correction was not pasted and
    /// is left on the clipboard (ADR 0002).
    LeftOnClipboard,
}

impl Notification {
    /// The text shown to the user.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Failed(cause) => format!("The proofread failed: {cause}. The selection is unchanged."),
            Self::LeftOnClipboard => {
                "The application changed during the proofread: the correction was not pasted, it is on the clipboard.".to_owned()
            }
        }
    }
}

/// The notification sink of the OS. The adapter shows the notification and holds no rule.
pub trait Notifier: Send + Sync + 'static {
    /// Shows the notification. Returns at once: a proofread never waits on the user.
    fn notify(&self, notification: &Notification);
}

/// The sink of an agent given none: the notifications go nowhere.
pub(crate) struct NoNotifications;

impl Notifier for NoNotifications {
    fn notify(&self, _notification: &Notification) {}
}
