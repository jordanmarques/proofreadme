//! The OS port of the agent on this system, and the keyboard hook that feeds it.

use proofreadme_core::Agent;

/// The OS port of this system.
#[cfg(windows)]
pub(crate) fn os() -> proofreadme_os_windows::WindowsOs {
    proofreadme_os_windows::WindowsOs
}

/// The OS port of this system: none yet.
#[cfg(not(windows))]
pub(crate) fn os() -> crate::UnwiredOs {
    crate::UnwiredOs
}

/// Starts passing the key events of the session to the agent. Failing it, the agent runs without a
/// trigger, and says why on the error output.
pub(crate) fn listen(agent: &Agent) {
    #[cfg(windows)]
    {
        let agent = agent.clone();
        if let Err(error) = proofreadme_os_windows::listen(move |event| agent.key_event(event)) {
            eprintln!("ProofReadMe: no trigger: {error}");
        }
    }
    #[cfg(not(windows))]
    let _ = agent;
}
