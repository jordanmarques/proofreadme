//! The registration of the agent at login (ADR 0001): the agent decides when it must be aligned
//! with the "Launch at login" switch, the platform executes it and reports back.

/// What the registration at login must do, as the agent asks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginItem {
    /// Register the agent: it starts with the session.
    Register,
    /// Remove the registration.
    Unregister,
}

impl LoginItem {
    /// What aligns the registration with this preference of the settings.
    pub(crate) fn for_preference(launch_at_login: bool) -> Self {
        if launch_at_login { Self::Register } else { Self::Unregister }
    }
}

/// Why the registration at login does not match the preference of the settings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoginError {
    /// The system refused to register the agent.
    #[error("registration at login failed: {0}")]
    Register(String),
    /// The system refused to remove the registration.
    #[error("removing the registration at login failed: {0}")]
    Unregister(String),
    /// The registration cannot even be prepared (executable not found, OS not supported).
    #[error("registration at login is impossible: {0}")]
    Unavailable(String),
}
