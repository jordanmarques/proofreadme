//! The registration of the agent at login: LaunchAgent on macOS, `Run` key of the user on Windows
//! (ADR 0001).
//!
//! The window only knows the [`LoginRegistration`] port. The real registration
//! ([`SystemRegistration`]) goes through the `auto-launch` crate; the tests inject a simulated
//! port and never touch the system.

use std::path::Path;

use auto_launch::{AutoLaunch, AutoLaunchBuilder, MacOSLaunchMode, WindowsEnableMode};

/// The command line argument put in the registration: launched with it, the agent starts with its
/// window hidden.
pub const BACKGROUND_ARGUMENT: &str = "--background";

/// The identifier of the application: the `Label` of the LaunchAgent (file
/// `~/Library/LaunchAgents/<identifier>.plist`) and the identifier of the macOS bundle.
pub const IDENTIFIER: &str = "io.github.jordanmarques.proofreadme";

/// What prevents registering the agent at login, or removing it.
#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    /// The system refused the operation, for the cause it reports.
    #[error("{0}")]
    System(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The running executable, the one that would have to be registered, cannot be found.
    #[error("the running executable cannot be found: {0}")]
    ExecutableNotFound(#[source] std::io::Error),
}

impl RegistrationError {
    /// The error of a refusal by the system, whatever the form in which it reports it.
    pub fn from_system(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::System(cause.into())
    }
}

/// The port through which the agent registers at login, or unregisters.
///
/// Both operations are idempotent: registering an already registered agent updates the
/// registration, removing an absent registration succeeds.
pub trait LoginRegistration {
    /// Registers the agent: it will be launched at login.
    ///
    /// # Errors
    ///
    /// The cause, to be shown to the user.
    fn register(&mut self) -> Result<(), RegistrationError>;

    /// Removes the registration.
    ///
    /// # Errors
    ///
    /// The cause, to be shown to the user.
    fn unregister(&mut self) -> Result<(), RegistrationError>;
}

/// The real registration of this executable with the OS.
pub struct SystemRegistration {
    registration: AutoLaunch,
}

impl SystemRegistration {
    /// Prepares the registration of the given executable, without writing anything.
    ///
    /// # Errors
    ///
    /// The cause, if the OS is not supported.
    pub fn for_executable(executable: &Path) -> Result<Self, RegistrationError> {
        let path = executable.to_string_lossy();
        // The `Run` key carries a command line: a path containing a space is put in quotes there,
        // which `auto-launch` does not do by itself.
        let path = if cfg!(windows) { format!("\"{path}\"") } else { path.into_owned() };
        // On Windows the name of the value of the `Run` key is the one the Task Manager shows; on
        // macOS it is the `Label` of the LaunchAgent.
        let name = if cfg!(windows) { crate::NAME } else { IDENTIFIER };
        AutoLaunchBuilder::new()
            .set_app_name(name)
            .set_app_path(&path)
            .set_args(&[BACKGROUND_ARGUMENT])
            .set_macos_launch_mode(MacOSLaunchMode::LaunchAgent)
            .set_bundle_identifiers(&[IDENTIFIER])
            .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
            .build()
            .map(|registration| Self { registration })
            .map_err(RegistrationError::from_system)
    }
}

impl LoginRegistration for SystemRegistration {
    fn register(&mut self) -> Result<(), RegistrationError> {
        self.registration.enable().map_err(RegistrationError::from_system)
    }

    fn unregister(&mut self) -> Result<(), RegistrationError> {
        self.registration.disable().map_err(RegistrationError::from_system)
    }
}

/// The registration to plug into the window of the application, `None` for a development build.
///
/// A development build (`cargo run`) never registers: its executable lives in `target/` and is not
/// meant to start with the session. Only a release build registers the running executable.
///
/// # Errors
///
/// The cause, if the running executable cannot be found or the OS is not supported.
pub fn system_registration() -> Result<Option<SystemRegistration>, RegistrationError> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }
    let executable = std::env::current_exe().map_err(RegistrationError::ExecutableNotFound)?;
    SystemRegistration::for_executable(&executable).map(Some)
}
