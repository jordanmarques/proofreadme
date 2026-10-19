//! The keychain of the OS (Keychain on macOS, Credential Manager on Windows) behind the keychain
//! port of the agent. No rule lives here.

use proofreadme_core::{Keychain, KeychainError};

/// The service under which the API key is stored: the identifier of the application.
const SERVICE: &str = crate::IDENTIFIER;

/// The account of the API key within the service.
const ACCOUNT: &str = "provider-api-key";

/// The keychain of the OS, through `keyring`.
#[derive(Debug, Default)]
pub struct SystemKeychain;

impl SystemKeychain {
    fn entry() -> Result<keyring::Entry, KeychainError> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(failure)
    }
}

impl Keychain for SystemKeychain {
    fn api_key(&mut self) -> Result<Option<String>, KeychainError> {
        match Self::entry()?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(failure(error)),
        }
    }

    fn set_api_key(&mut self, key: &str) -> Result<(), KeychainError> {
        Self::entry()?.set_password(key).map_err(failure)
    }

    fn delete_api_key(&mut self) -> Result<(), KeychainError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(failure(error)),
        }
    }
}

/// The cause of a refusal of the keychain. The messages of `keyring` name the store and the
/// error, never the secret.
fn failure(error: keyring::Error) -> KeychainError {
    KeychainError(error.to_string())
}
