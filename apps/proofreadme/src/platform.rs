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
pub(crate) fn os() -> crate::UnsupportedOs {
    crate::UnsupportedOs
}

/// Starts passing the key events of the session to the agent. Failing it, the agent runs without a
/// trigger: the hook reports it, and the agent shows it.
pub(crate) fn listen(agent: &Agent) {
    let hook = agent.hook();
    #[cfg(windows)]
    {
        let feed = hook.clone();
        if let Err(error) = proofreadme_os_windows::listen(move |event| feed.key_event(event)) {
            hook.failed(&error);
        }
    }
    // On macOS the hook also reports the Accessibility permission to the agent.
    #[cfg(target_os = "macos")]
    if let Err(error) = proofreadme_os_macos::listen(hook.clone()) {
        hook.failed(&error);
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = hook;
}

/// Opens the pane of System Settings where the permission of the keyboard hook is granted: only
/// macOS asks for one. A pane that does not open is told in the agent log.
#[cfg(target_os = "macos")]
pub(crate) fn permission_pane(agent: &Agent) -> Option<impl FnMut() + 'static> {
    let agent = agent.clone();
    Some(move || {
        if let Err(error) = proofreadme_os_macos::open_permission_pane() {
            agent.note(&format!("System Settings could not open: {error}"));
        }
    })
}

/// Opens the pane of the system where the permission of the keyboard hook is granted: none here.
#[cfg(not(target_os = "macos"))]
pub(crate) fn permission_pane(_agent: &Agent) -> Option<fn()> {
    None
}
