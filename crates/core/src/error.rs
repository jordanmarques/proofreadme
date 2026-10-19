//! Why a proofread failed. No variant carries the selection, the correction or the API key.

use crate::os::OsError;

/// The cause of a failed proofread.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProofreadError {
    /// The provider could not be reached.
    #[error("provider unreachable: {0}")]
    Network(String),
    /// The provider responded with an error status (a refused key is 401 or 403).
    #[error("provider error: HTTP {0}")]
    Status(u16),
    /// The provider did not respond within the timeout.
    #[error("no response from the provider within the timeout")]
    Timeout,
    /// The response of the provider is not a chat completion with a text content.
    #[error("unreadable response from the provider")]
    MalformedResponse,
    /// The correction is empty once cleaned.
    #[error("empty correction")]
    EmptyCorrection,
    /// The OS refused to start a thread of the agent, or a thread ended without reporting.
    #[error("thread of the agent: {0}")]
    Thread(String),
    /// The OS refused a clipboard or keystroke operation.
    #[error(transparent)]
    Os(#[from] OsError),
}

impl ProofreadError {
    /// The thread of the request to the provider ended without a response: it panicked.
    pub(crate) fn request_thread_ended() -> Self {
        Self::Thread("the request thread ended without a response".to_owned())
    }
}
