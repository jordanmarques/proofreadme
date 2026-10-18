//! Where the settings are kept: a TOML config file whose location the caller supplies, written
//! atomically, and the keychain of the OS for the API key, which never goes into the file.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::os::Modifier;
use crate::settings::{Provider, Settings, SettingsError};

/// A failure of the keychain. Its detail never carries the API key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct KeychainError(pub String);

/// The keychain of the OS, holding the API key of the provider. The platform implements it with
/// the system keychain; [`MemoryKeychain`] keeps it in memory.
pub trait Keychain: Send + 'static {
    /// The API key, `None` if none is stored.
    fn api_key(&mut self) -> Result<Option<String>, KeychainError>;

    /// Stores the API key, replacing the previous one.
    fn set_api_key(&mut self, key: &str) -> Result<(), KeychainError>;

    /// Removes the API key. Removing an absent key succeeds.
    fn delete_api_key(&mut self) -> Result<(), KeychainError>;
}

/// A keychain in memory. Its clones share the same key.
#[derive(Debug, Clone, Default)]
pub struct MemoryKeychain {
    key: Arc<Mutex<Option<String>>>,
}

impl MemoryKeychain {
    /// A keychain already holding this key.
    pub fn with_key(key: &str) -> Self {
        Self { key: Arc::new(Mutex::new(Some(key.to_owned()))) }
    }

    /// The key it holds.
    pub fn key(&self) -> Option<String> {
        self.key.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

impl Keychain for MemoryKeychain {
    fn api_key(&mut self) -> Result<Option<String>, KeychainError> {
        Ok(self.key())
    }

    fn set_api_key(&mut self, key: &str) -> Result<(), KeychainError> {
        *self.key.lock().unwrap_or_else(PoisonError::into_inner) = Some(key.to_owned());
        Ok(())
    }

    fn delete_api_key(&mut self) -> Result<(), KeychainError> {
        *self.key.lock().unwrap_or_else(PoisonError::into_inner) = None;
        Ok(())
    }
}

/// Where an agent keeps its settings: the config file and the keychain.
pub struct Storage {
    config: PathBuf,
    keychain: Box<dyn Keychain>,
}

impl Storage {
    /// The settings kept in the config file at this location and the key in this keychain.
    pub fn new(config: impl Into<PathBuf>, keychain: impl Keychain) -> Self {
        Self { config: config.into(), keychain: Box::new(keychain) }
    }

    /// True if the config file exists: false at first launch.
    pub(crate) fn exists(&self) -> bool {
        self.config.exists()
    }

    /// Reads the settings. A missing or unreadable file gives the defaults, a missing key an empty
    /// one; an unreadable keychain is returned beside the settings.
    pub(crate) fn load(&mut self) -> (Settings, Option<KeychainError>) {
        let config: Config = fs::read_to_string(&self.config).ok().and_then(|text| toml::from_str(&text).ok()).unwrap_or_default();
        let mut settings = config.settings();
        let failure = match self.keychain.api_key() {
            Ok(key) => {
                settings.provider.api_key = key.unwrap_or_default();
                None
            }
            Err(error) => Some(error),
        };
        (settings, failure)
    }

    /// Writes the config file, then the key to the keychain (an empty key removes it): a config
    /// that cannot be written leaves the keychain untouched.
    pub(crate) fn save(&mut self, settings: &Settings) -> Result<(), SettingsError> {
        save_atomically(&self.config, &Config::from(settings)).map_err(|error| SettingsError::ConfigNotSaved(error.to_string()))?;
        let key = settings.provider.api_key.trim();
        let stored = if key.is_empty() { self.keychain.delete_api_key() } else { self.keychain.set_api_key(key) };
        stored.map_err(|error| SettingsError::KeyNotSaved(error.0))
    }
}

/// The trigger key as written in the config.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum TriggerKey {
    Ctrl,
    Alt,
    Shift,
    Cmd,
}

impl From<Modifier> for TriggerKey {
    fn from(modifier: Modifier) -> Self {
        match modifier {
            Modifier::Ctrl => Self::Ctrl,
            Modifier::Alt => Self::Alt,
            Modifier::Shift => Self::Shift,
            Modifier::Cmd => Self::Cmd,
        }
    }
}

impl From<TriggerKey> for Modifier {
    fn from(key: TriggerKey) -> Self {
        match key {
            TriggerKey::Ctrl => Self::Ctrl,
            TriggerKey::Alt => Self::Alt,
            TriggerKey::Shift => Self::Shift,
            TriggerKey::Cmd => Self::Cmd,
        }
    }
}

/// The config file. Every field is optional: a missing one takes its default.
#[derive(Debug, Default, Deserialize, Serialize)]
struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    trigger: Option<TriggerKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timeout_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reasoning: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    launch_at_login: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
}

impl Config {
    /// The settings it describes, with no API key.
    fn settings(self) -> Settings {
        let defaults = Settings::default();
        Settings {
            trigger: self.trigger.map_or(defaults.trigger, Modifier::from),
            provider: Provider {
                base_url: self.base_url.unwrap_or(defaults.provider.base_url),
                api_key: String::new(),
                model: self.model.unwrap_or(defaults.provider.model),
            },
            prompt: self.prompt.unwrap_or(defaults.prompt),
            reasoning: self.reasoning.unwrap_or(defaults.reasoning),
            timeout: self.timeout_seconds.map_or(defaults.timeout, Duration::from_secs),
            launch_at_login: self.launch_at_login.unwrap_or(defaults.launch_at_login),
        }
    }
}

impl From<&Settings> for Config {
    /// Everything but the API key.
    fn from(settings: &Settings) -> Self {
        Self {
            base_url: Some(settings.provider.base_url.clone()),
            model: Some(settings.provider.model.clone()),
            trigger: Some(settings.trigger.into()),
            timeout_seconds: Some(settings.timeout.as_secs()),
            reasoning: Some(settings.reasoning),
            launch_at_login: Some(settings.launch_at_login),
            prompt: Some(settings.prompt.clone()),
        }
    }
}

/// Writes the config, creating its folder if needed. The write goes through a neighbouring file,
/// renamed once complete: a power cut leaves the old config or the new one, never a truncated file.
fn save_atomically(path: &Path, config: &Config) -> io::Result<()> {
    let text = toml::to_string(config).map_err(io::Error::other)?;
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let temporary = temporary_path(path);
    let written = write(&temporary, &text).and_then(|()| fs::rename(&temporary, path));
    if written.is_err() {
        // The temporary file is removed on a best-effort basis; the original error prevails.
        let _ = fs::remove_file(&temporary);
    }
    written
}

/// The neighbouring file of the config, in the same folder so that the rename stays on the same
/// volume.
fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Writes a whole file and waits for it to be on the disk.
fn write(path: &Path, text: &str) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()
}
