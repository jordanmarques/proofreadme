//! Why a proofread failed. No variant carries the selection, the correction or the API key.

use crate::os::OsError;

/// The cause of a failed proofread.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProofreadError {
    /// The provider could not be reached.
    #[error("provider unreachable: {0}")]
    Network(String),
    /// The provider answered with an error status (a refused key is 401 or 403).
    #[error("provider error: HTTP {0}")]
    Status(u16),
    /// The provider did not answer within the timeout.
    #[error("no answer from the provider within the timeout")]
    Timeout,
    /// The answer of the provider is not a chat completion with a text content.
    #[error("unreadable answer from the provider")]
    MalformedAnswer,
    /// The correction is empty once cleaned.
    #[error("empty correction")]
    EmptyCorrection,
    /// The OS refused to start a thread of the agent.
    #[error("cannot start a thread: {0}")]
    Thread(String),
    /// The OS refused a clipboard or keystroke operation.
    #[error(transparent)]
    Os(#[from] OsError),
}
