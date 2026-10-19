//! The OS port of the agent on this system, the keyboard hook that feeds it, and the pane of the
//! system where the permission of the hook is granted.

use proofreadme_core::Agent;

/// The OS port of this system.
#[cfg(windows)]
pub(crate) fn os() -> proofreadme_os_windows::WindowsOs {
    proofreadme_os_windows::WindowsOs
}

/// The OS port of this system.
#[cfg(target_os = "macos")]
pub(crate) fn os() -> proofreadme_os_macos::MacOs {
    proofreadme_os_macos::MacOs::new()
}

/// The OS port of this system: none.
#[cfg(not(any(windows, target_os = "macos")))]
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
    // On macOS the hook also reports the Accessibility permission to the agent.
    #[cfg(target_os = "macos")]
    if let Err(error) = proofreadme_os_macos::listen(agent.hook()) {
        eprintln!("ProofReadMe: no trigger: {error}");
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = agent;
}

/// Opens the pane of System Settings where the permission of the keyboard hook is granted: only
/// macOS asks for one.
#[cfg(target_os = "macos")]
pub(crate) fn permission_pane() -> Option<impl FnMut() + 'static> {
    Some(|| {
        if let Err(error) = proofreadme_os_macos::open_permission_pane() {
            eprintln!("ProofReadMe: System Settings could not open: {error}");
        }
    })
}

/// Opens the pane of the system where the permission of the keyboard hook is granted: none here.
#[cfg(not(target_os = "macos"))]
pub(crate) fn permission_pane() -> Option<fn()> {
    None
}
