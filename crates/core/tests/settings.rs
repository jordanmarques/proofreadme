//! The settings of the agent: their defaults, their file, the API key in the keychain, and the
//! rules of a save.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use proofreadme_core::{
    Agent, Keychain, KeychainError, MemoryKeychain, Modifier, PLACEHOLDER, ProofreadError, Provider, ProviderTest, Settings, SettingsError, Storage,
    TEST_SENTENCE,
};
use support::{API_KEY, DELAY, Desktop, Keyboard, LocalProvider, Reply, timing};

/// The location of the config in a data folder simulated in `folder`.
fn config(folder: &Path) -> PathBuf {
    folder.join("ProofReadMe").join("config.toml")
}

fn load(desktop: &Desktop, folder: &Path, keychain: &MemoryKeychain) -> Agent {
    Agent::load(desktop.clone(), Storage::new(config(folder), keychain.clone()), timing())
}

/// Settings that differ from the defaults in every field.
fn changed(provider: &LocalProvider) -> Settings {
    Settings {
        trigger: Modifier::Shift,
        provider: Provider { base_url: provider.base_url(), api_key: API_KEY.to_owned(), model: "other/model".to_owned() },
        prompt: format!("Fix: {PLACEHOLDER}"),
        reasoning: false,
        timeout: Duration::from_secs(12),
        launch_at_login: false,
    }
}

#[test]
fn the_defaults_are_those_of_the_spec() {
    let settings = Settings::default();

    assert_eq!(settings.provider.base_url, "https://openrouter.ai/api/v1");
    assert_eq!(settings.provider.model, "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free");
    assert_eq!(settings.provider.api_key, "");
    assert_eq!(settings.trigger, if cfg!(target_os = "macos") { Modifier::Cmd } else { Modifier::Ctrl });
    assert_eq!(settings.timeout, Duration::from_secs(90));
    assert!(settings.reasoning);
    assert!(settings.launch_at_login);
    assert!(settings.prompt.starts_with("Tu es correcteur. Le texte à relire se trouve entre les balises <texte> et </texte>.\n\n"));
    assert!(settings.prompt.contains("## Texte en français : correction minimale\nCorrige uniquement :\n"));
    assert!(settings.prompt.contains("- Les variables et balises : {{nom}}, {nom}, <balise>, $VAR.\n"));
    assert!(settings.prompt.ends_with("S'il n'y a aucune faute, renvoie le texte à l'identique.\n\n<texte>\n{{TEXTE}}\n</texte>"));
}

#[test]
fn at_first_launch_the_agent_runs_with_the_defaults_and_asks_for_the_settings() {
    let folder = tempfile::tempdir().unwrap();

    let agent = load(&Desktop::new(), folder.path(), &MemoryKeychain::default());

    assert_eq!(agent.settings(), Settings::default());
    assert_eq!(agent.state().settings_requests, 1);
    assert!(!config(folder.path()).exists(), "nothing is written before a save");
}

#[test]
fn saved_settings_come_back_at_the_next_launch_and_the_key_stays_out_of_the_file() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    let keychain = MemoryKeychain::default();
    let agent = load(&Desktop::new(), folder.path(), &keychain);

    agent.save_settings(changed(&provider)).unwrap();

    assert_eq!(agent.settings(), changed(&provider));
    assert_eq!(keychain.key(), Some(API_KEY.to_owned()));
    let text = fs::read_to_string(config(folder.path())).unwrap();
    assert!(!text.contains(API_KEY), "the key is not in the config");
    let reloaded = load(&Desktop::new(), folder.path(), &keychain);
    assert_eq!(reloaded.settings(), changed(&provider));
    assert_eq!(reloaded.state().settings_requests, 0, "a configured agent does not ask for the settings");
}

#[test]
fn an_empty_key_removes_it_from_the_keychain() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    let keychain = MemoryKeychain::with_key(API_KEY);
    let agent = load(&Desktop::new(), folder.path(), &keychain);
    let mut settings = changed(&provider);
    settings.provider.api_key = String::new();

    agent.save_settings(settings).unwrap();

    assert_eq!(keychain.key(), None);
}

#[test]
fn a_prompt_without_the_placeholder_is_refused_and_nothing_is_written() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    let keychain = MemoryKeychain::default();
    let agent = load(&Desktop::new(), folder.path(), &keychain);
    let mut settings = changed(&provider);
    settings.prompt = "Corrige ce texte.".to_owned();

    let refused = agent.save_settings(settings);

    assert_eq!(refused, Err(SettingsError::MissingPlaceholder));
    assert_eq!(agent.state().settings_error, Some(SettingsError::MissingPlaceholder));
    assert!(!config(folder.path()).exists());
    assert_eq!(keychain.key(), None);
    assert_eq!(agent.settings(), Settings::default());
}

#[test]
fn a_successful_save_clears_the_last_refusal() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    let agent = load(&Desktop::new(), folder.path(), &MemoryKeychain::default());
    let mut refused = changed(&provider);
    refused.prompt = String::new();
    let _ = agent.save_settings(refused);

    agent.save_settings(changed(&provider)).unwrap();

    assert_eq!(agent.state().settings_error, None);
}

#[test]
fn a_config_that_cannot_be_written_is_a_refusal_and_the_settings_stay() {
    let folder = tempfile::tempdir().unwrap();
    // A file where the folder of the config should be: the folder cannot be created.
    fs::write(folder.path().join("ProofReadMe"), "").unwrap();
    let provider = LocalProvider::start();
    let keychain = MemoryKeychain::default();
    let agent = load(&Desktop::new(), folder.path(), &keychain);

    let refused = agent.save_settings(changed(&provider));

    assert!(matches!(refused, Err(SettingsError::ConfigNotSaved(_))), "{refused:?}");
    assert_eq!(agent.settings(), Settings::default());
    assert_eq!(keychain.key(), None, "the keychain is left untouched");
}

#[test]
fn a_new_trigger_key_takes_effect_at_once() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    let desktop = Desktop::new();
    desktop.select("Helo");
    let agent = load(&desktop, folder.path(), &MemoryKeychain::default());

    agent.save_settings(changed(&provider)).unwrap();
    Keyboard::new().double_press(&agent, Modifier::Shift);

    assert_eq!(provider.next_request().body["model"], "other/model");
    assert!(agent.wait_until_idle(DELAY));
}

#[test]
fn a_trigger_without_a_key_asks_for_the_settings_and_sends_no_request() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();
    let desktop = Desktop::new();
    desktop.select("Helo");
    let agent = load(&desktop, folder.path(), &MemoryKeychain::default());
    let mut settings = changed(&provider);
    settings.provider.api_key = String::new();
    agent.save_settings(settings).unwrap();
    let before = agent.state().settings_requests;

    Keyboard::new().double_press(&agent, Modifier::Shift);
    assert!(agent.wait_until_idle(DELAY));

    assert_eq!(agent.state().settings_requests, before + 1);
    assert!(desktop.keystrokes().is_empty(), "the selection is not even copied");
    assert!(provider.received().is_empty());
}

/// A keychain the OS refuses to open.
struct LockedKeychain;

impl Keychain for LockedKeychain {
    fn api_key(&mut self) -> Result<Option<String>, KeychainError> {
        Err(KeychainError("locked".to_owned()))
    }

    fn set_api_key(&mut self, _key: &str) -> Result<(), KeychainError> {
        Err(KeychainError("locked".to_owned()))
    }

    fn delete_api_key(&mut self) -> Result<(), KeychainError> {
        Err(KeychainError("locked".to_owned()))
    }
}

#[test]
fn a_locked_keychain_is_reported_at_launch_and_at_save() {
    let folder = tempfile::tempdir().unwrap();
    let provider = LocalProvider::start();

    let agent = Agent::load(Desktop::new(), Storage::new(config(folder.path()), LockedKeychain), timing());

    assert_eq!(agent.state().settings_error, Some(SettingsError::KeyUnreadable("locked".to_owned())));
    assert_eq!(agent.state().settings_requests, 1);
    assert_eq!(agent.save_settings(changed(&provider)), Err(SettingsError::KeyNotSaved("locked".to_owned())));
    assert_eq!(agent.settings(), Settings::default());
}

/// An agent in memory whose changes of state are signalled on the returned channel.
fn observed() -> (Agent, Receiver<()>) {
    let (sender, wakeups) = mpsc::channel();
    let agent = support::agent(&Desktop::new(), Settings::default()).on_change(move || {
        let _ = sender.send(());
    });
    (agent, wakeups)
}

/// Waits, on the wakeups of the agent, until its Test of the provider is no longer running.
fn tested(agent: &Agent, wakeups: &Receiver<()>) -> Option<ProviderTest> {
    loop {
        let test = agent.state().provider_test;
        if test != Some(ProviderTest::Running) {
            return test;
        }
        wakeups.recv_timeout(DELAY).expect("the Test did not end in time");
    }
}

#[test]
fn the_test_sends_the_test_sentence_through_the_given_settings_and_shows_the_correction() {
    let provider = LocalProvider::start();
    let (reply, release) = Reply::correction("Je suis allé au marché hier.").held();
    provider.reply(reply);
    let (agent, wakeups) = observed();

    agent.test_provider(&changed(&provider)).unwrap();

    assert_eq!(agent.state().provider_test, Some(ProviderTest::Running), "the request runs off the calling thread");
    let request = provider.next_request();
    assert_eq!(request.authorization.as_deref(), Some("Bearer sk-test-key"));
    assert_eq!(request.body["messages"][0]["content"], format!("Fix: {TEST_SENTENCE}"));
    release.send(()).unwrap();
    assert_eq!(tested(&agent, &wakeups), Some(ProviderTest::Corrected("Je suis allé au marché hier.".to_owned())));
    assert_eq!(agent.settings(), Settings::default(), "a Test saves nothing");
}

#[test]
fn a_failed_test_shows_its_cause() {
    let provider = LocalProvider::start();
    provider.reply(Reply::status(401));
    let (agent, wakeups) = observed();

    agent.test_provider(&changed(&provider)).unwrap();

    assert_eq!(tested(&agent, &wakeups), Some(ProviderTest::Failed(ProofreadError::Status(401))));
}

#[test]
fn a_test_with_a_prompt_without_the_placeholder_is_refused_and_sends_nothing() {
    let provider = LocalProvider::start();
    let (agent, _wakeups) = observed();
    let mut settings = changed(&provider);
    settings.prompt = "Corrige.".to_owned();

    assert_eq!(agent.test_provider(&settings), Err(SettingsError::MissingPlaceholder));

    assert_eq!(agent.state().provider_test, None);
    assert_eq!(agent.state().settings_error, Some(SettingsError::MissingPlaceholder));
    assert!(provider.received().is_empty());
}
