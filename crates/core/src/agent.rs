//! The agent: it receives the key events of the OS, detects the trigger and runs one proofread at
//! a time on a thread of its own, publishing its state as it goes.

use std::fmt::Display;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::ProofreadError;
use crate::log::AgentLog;
use crate::login::{LoginError, LoginItem};
use crate::notification::{NoNotifications, Notification, Notifier};
use crate::os::{KeyEvent, Os};
use crate::proofread::{Outcome, Signal};
use crate::settings::{Settings, SettingsError};
use crate::storage::Storage;
use crate::trigger::Detector;
use crate::wait::{self, lock};
use crate::{proofread, provider};

/// The step a running proofread is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Saving the clipboard and copying the selection.
    Capturing,
    /// Waiting for the correction from the provider.
    Requesting,
    /// Pasting the correction and restoring the clipboard.
    Replacing,
}

/// What the agent is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// Listening for the trigger.
    Idle,
    /// Running a proofread.
    Proofreading(Step),
}

/// The sentence with mistakes that the Test of the provider sends through the proofreading
/// prompt.
///
/// In French and English on purpose, an agreed exception to the English of the repository (see
/// `CODING_STANDARDS.md`): the default proofreading prompt treats the two languages differently,
/// and the Test exercises both.
pub const TEST_SENTENCE: &str = "Je suis aller au marcher hier, and it were a realy nice day.";

/// Where the Test of the provider is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderTest {
    /// The test sentence is with the provider.
    Running,
    /// The provider returned this correction of the test sentence.
    Corrected(String),
    /// The request failed, for this cause.
    Failed(ProofreadError),
}

/// Why the keyboard hook of the platform cannot listen: no trigger reaches the agent.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("no trigger: {0}")]
pub struct NoTrigger(pub String);

/// What the agent shows of itself: the window and the icon render it as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// What the agent is doing.
    pub activity: Activity,
    /// The cause of the last failed proofread, cleared when a proofread starts. Its notification
    /// told it already.
    pub last_failure: Option<ProofreadError>,
    /// How many times the agent asked for the settings to be shown: at a first launch, and at each
    /// trigger without a usable provider. The window opens the settings each time it grows.
    pub settings_requests: u64,
    /// How many times the agent asked for its window to come back: each time it becomes deaf to
    /// the trigger (a permission lost, a hook that cannot listen). The window shows itself each time
    /// it grows.
    pub window_requests: u64,
    /// Why the last save (or Test) of the settings was refused, or why the key could not be read
    /// at launch. Cleared by a successful save.
    pub settings_error: Option<SettingsError>,
    /// The last Test of the provider, if any.
    pub provider_test: Option<ProviderTest>,
    /// Whether the keyboard hook has the permission it needs, as the platform reports it.
    pub permission: Permission,
    /// Why the keyboard hook cannot listen at all, if it cannot.
    pub no_trigger: Option<NoTrigger>,
    /// Why the registration at login does not match the preference of the settings, if it does
    /// not.
    pub login_error: Option<LoginError>,
}

/// The permission of the OS the keyboard hook needs to see the trigger: Accessibility (or Input
/// Monitoring) on macOS. Windows asks for none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// The hook listens.
    Granted,
    /// The permission is missing: no trigger reaches the agent until the user grants it.
    Missing,
    /// The permission was granted, but the OS lets the hook listen only after a relaunch of the
    /// agent.
    RelaunchNeeded,
}

/// The look of the session icon of the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    /// Listening for the trigger.
    Idle,
    /// Running a proofread.
    Running,
    /// Deaf to the trigger: the permission is missing ([`State::permission`]) or the hook cannot
    /// listen ([`State::no_trigger`]). A failed proofread is told by its notification, not here.
    Alert,
}

impl State {
    fn initial() -> Self {
        Self {
            activity: Activity::Idle,
            last_failure: None,
            settings_requests: 0,
            window_requests: 0,
            settings_error: None,
            provider_test: None,
            permission: Permission::Granted,
            no_trigger: None,
            login_error: None,
        }
    }

    /// True if no trigger reaches the agent: the permission is missing or the hook cannot listen.
    #[must_use]
    pub fn is_deaf(&self) -> bool {
        self.permission != Permission::Granted || self.no_trigger.is_some()
    }

    /// The look of the session icon for this state.
    #[must_use]
    pub fn icon(&self) -> IconState {
        match self.activity {
            Activity::Proofreading(_) => IconState::Running,
            Activity::Idle if self.is_deaf() => IconState::Alert,
            Activity::Idle => IconState::Idle,
        }
    }

    /// True if the menu of the session icon offers to cancel: only while a proofread runs.
    #[must_use]
    pub fn can_cancel(&self) -> bool {
        matches!(self.activity, Activity::Proofreading(_))
    }
}

/// The delays of the agent around the copy and the paste, which the applications perform
/// asynchronously.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// How long the agent waits for the clipboard to change after the synthesized copy before
    /// deciding that nothing is selected.
    pub copy_wait: Duration,
    /// How long the agent leaves the correction on the clipboard after the synthesized paste,
    /// for the application to read it, before restoring the saved clipboard.
    pub paste_settle: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self { copy_wait: Duration::from_millis(1000), paste_settle: Duration::from_millis(400) }
    }
}

/// What the agent and the thread of a proofread share.
pub(crate) struct Shared {
    control: Mutex<Control>,
    /// Signalled at each change of the state.
    changed: Condvar,
    /// Called at each change of the state (the window repaints).
    wakeup: Mutex<Option<Wakeup>>,
    /// The OS, used by one proofread at a time.
    os: Mutex<Box<dyn Os>>,
    pub(crate) timing: Timing,
    /// Where the notifications go.
    notifier: Mutex<Arc<dyn Notifier>>,
    /// Where the failures are written; `None` writes them nowhere.
    log: Mutex<Option<AgentLog>>,
}

/// What the agent calls at each change of its state.
type Wakeup = Arc<dyn Fn() + Send + Sync>;

struct Control {
    state: State,
    settings: Settings,
    detector: Detector,
    /// Where the settings are saved; `None` keeps them in memory only.
    storage: Option<Storage>,
    /// How to reach the running proofread, to cancel it.
    running: Option<Sender<Signal>>,
    /// What the registration at login must do, until the platform takes it.
    login_item: Option<LoginItem>,
}

impl Shared {
    /// The control part, even if a thread panicked while holding it.
    fn control(&self) -> MutexGuard<'_, Control> {
        lock(&self.control)
    }

    /// The OS, even if a thread panicked while holding it.
    pub(crate) fn os(&self) -> MutexGuard<'_, Box<dyn Os>> {
        lock(&self.os)
    }

    /// Tells the waiting threads and the observer that the state changed.
    fn notify(&self) {
        self.changed.notify_all();
        let wakeup = lock(&self.wakeup).clone();
        if let Some(wakeup) = wakeup {
            wakeup();
        }
    }

    /// Changes the control part, then tells the change.
    fn change(&self, change: impl FnOnce(&mut Control)) {
        change(&mut self.control());
        self.notify();
    }

    /// Shows a notification.
    fn notify_user(&self, notification: &Notification) {
        let notifier = Arc::clone(&lock(&self.notifier));
        notifier.notify(notification);
    }

    /// The agent log, if any.
    fn log(&self) -> Option<AgentLog> {
        lock(&self.log).clone()
    }

    /// Tells the failure of a proofread: a notification and a line of the agent log.
    fn report_failure(&self, cause: &ProofreadError, model: &str, duration: Duration) {
        self.notify_user(&Notification::Failed(cause.clone()));
        if let Some(log) = self.log() {
            // The notification already told the failure: a log that cannot be written has nowhere
            // else to say so.
            let _ = log.failure(cause, model, duration);
        }
    }

    /// Moves the running proofread to the given step.
    pub(crate) fn step(&self, step: Step) {
        self.change(|control| control.state.activity = Activity::Proofreading(step));
    }

    /// Ends the running proofread, with its failure if it failed.
    fn end(&self, failure: Option<ProofreadError>) {
        self.change(|control| {
            control.state.activity = Activity::Idle;
            control.state.last_failure = failure;
            control.running = None;
        });
    }

    /// Takes a key event: see [`Agent::key_event`].
    fn key_event(self: &Arc<Self>, event: KeyEvent) {
        let mut control = self.control();
        let trigger = control.settings.trigger;
        if !control.detector.feed(&event, trigger) || control.state.activity != Activity::Idle {
            return;
        }
        // Without a usable provider, the trigger shows the settings instead of sending a request.
        if !control.settings.provider.is_usable() {
            control.state.settings_requests = control.state.settings_requests.saturating_add(1);
            drop(control);
            self.notify();
            return;
        }
        // The guard is taken here, on the trigger: a second trigger during the capture is ignored.
        control.state.activity = Activity::Proofreading(Step::Capturing);
        control.state.last_failure = None;
        let settings = control.settings.clone();
        let model = settings.provider.model.clone();
        let (sender, signals) = mpsc::channel();
        control.running = Some(sender.clone());
        drop(control);
        self.notify();
        let shared = Arc::clone(self);
        let spawned = thread::Builder::new().name("proofread".to_owned()).spawn(move || {
            let started = Instant::now();
            let mut ending = Ending { shared: Arc::clone(&shared), failure: None };
            match proofread::run(&shared, &settings, &signals, &sender) {
                Ok(Outcome::Finished | Outcome::Cancelled) => {}
                Ok(Outcome::LeftOnClipboard) => shared.notify_user(&Notification::LeftOnClipboard),
                Err(cause) => {
                    shared.report_failure(&cause, &settings.provider.model, started.elapsed());
                    ending.failure = Some(cause);
                }
            }
        });
        if let Err(error) = spawned {
            let cause = ProofreadError::Thread(error.to_string());
            self.report_failure(&cause, &model, Duration::ZERO);
            self.end(Some(cause));
        }
    }
}

/// Ends the proofread when its thread ends, including if it panics: the agent never stays stuck
/// in a proofread that no longer runs.
struct Ending {
    shared: Arc<Shared>,
    failure: Option<ProofreadError>,
}

impl Drop for Ending {
    fn drop(&mut self) {
        self.shared.end(self.failure.take());
    }
}

/// The headless agent. A clone is a handle on the same agent.
#[derive(Clone)]
pub struct Agent {
    shared: Arc<Shared>,
}

impl Agent {
    /// An agent listening for the trigger of the settings, with the default timing.
    pub fn new(os: impl Os, settings: Settings) -> Self {
        Self::with_timing(os, settings, Timing::default())
    }

    /// An agent with the given delays around the copy and the paste, whose settings live in
    /// memory only.
    pub fn with_timing(os: impl Os, settings: Settings, timing: Timing) -> Self {
        Self::build(os, settings, None, State::initial(), timing)
    }

    /// An agent running with the settings of the storage. At a first launch (no config file yet),
    /// or without a usable provider, it asks for the settings at once. It asks at once, too, for
    /// the registration at login to be aligned with its settings.
    pub fn load(os: impl Os, mut storage: Storage, timing: Timing) -> Self {
        let first_launch = !storage.exists();
        let (settings, unreadable) = storage.load();
        let mut state = State::initial();
        if first_launch || !settings.provider.is_usable() {
            state.settings_requests = 1;
        }
        state.settings_error = unreadable.map(|error| SettingsError::KeyUnreadable(error.0));
        let agent = Self::build(os, settings, Some(storage), state, timing);
        agent.shared.change(|control| control.login_item = Some(LoginItem::for_preference(control.settings.launch_at_login)));
        agent
    }

    fn build(os: impl Os, settings: Settings, storage: Option<Storage>, state: State, timing: Timing) -> Self {
        let control = Control { state, settings, detector: Detector::new(), storage, running: None, login_item: None };
        let shared = Shared {
            control: Mutex::new(control),
            changed: Condvar::new(),
            wakeup: Mutex::new(None),
            os: Mutex::new(Box::new(os)),
            timing,
            notifier: Mutex::new(Arc::new(NoNotifications)),
            log: Mutex::new(None),
        };
        Self { shared: Arc::new(shared) }
    }

    /// Calls `wakeup` at each change of the state, from whichever thread made it.
    #[must_use]
    pub fn on_change(self, wakeup: impl Fn() + Send + Sync + 'static) -> Self {
        *lock(&self.shared.wakeup) = Some(Arc::new(wakeup));
        self
    }

    /// Sends the notifications of the agent (failures, correction left on the clipboard) to this
    /// sink.
    #[must_use]
    pub fn with_notifier(self, notifier: impl Notifier) -> Self {
        *lock(&self.shared.notifier) = Arc::new(notifier);
        self
    }

    /// Writes one line per failed proofread, and per problem that does not stop the agent, to
    /// this log.
    #[must_use]
    pub fn with_log(self, log: AgentLog) -> Self {
        *lock(&self.shared.log) = Some(log);
        self
    }

    /// The current state.
    pub fn state(&self) -> State {
        self.shared.control().state.clone()
    }

    /// The settings in force, API key included.
    pub fn settings(&self) -> Settings {
        self.shared.control().settings.clone()
    }

    /// Saves the settings, which take effect at once (a running proofread keeps those it started
    /// with). A proofreading prompt without the placeholder is refused and nothing is written; a
    /// refusal is also kept in the state. A saved change of "Launch at login" asks for the
    /// registration to be aligned ([`Agent::take_login_item`]).
    pub fn save_settings(&self, settings: Settings) -> Result<(), SettingsError> {
        let mut control = self.shared.control();
        let saved = settings.check().and_then(|()| control.storage.as_mut().map_or(Ok(()), |storage| storage.save(&settings)));
        if saved.is_ok() {
            if settings.launch_at_login != control.settings.launch_at_login {
                control.login_item = Some(LoginItem::for_preference(settings.launch_at_login));
            }
            control.settings = settings;
        }
        control.state.settings_error = saved.clone().err();
        drop(control);
        self.shared.notify();
        saved
    }

    /// What the registration at login must do, if the agent asks for it: the platform takes it
    /// once, executes it and reports with [`Agent::login_item_done`].
    pub fn take_login_item(&self) -> Option<LoginItem> {
        self.shared.control().login_item.take()
    }

    /// The outcome of the registration at login asked by [`Agent::take_login_item`]: a failure is
    /// kept in [`State::login_error`], a success clears it.
    pub fn login_item_done(&self, item: LoginItem, outcome: Result<(), &dyn Display>) {
        let error = outcome.err().map(|cause| match item {
            LoginItem::Register => LoginError::Register(cause.to_string()),
            LoginItem::Unregister => LoginError::Unregister(cause.to_string()),
        });
        self.shared.change(|control| control.state.login_error = error);
    }

    /// The registration at login cannot even be prepared, for this cause: the state says so.
    pub fn login_unavailable(&self, cause: &dyn Display) {
        self.shared.change(|control| control.state.login_error = Some(LoginError::Unavailable(cause.to_string())));
    }

    /// Writes a problem that does not stop the agent (a platform adapter that failed) to the agent
    /// log. Its text must not carry the text of the user.
    pub fn note(&self, problem: &dyn Display) {
        if let Some(log) = self.shared.log() {
            log.note(problem);
        }
    }

    /// Sends [`TEST_SENTENCE`] through these settings, on a thread of its own; the outcome lands in
    /// [`State::provider_test`]. Ignored while a Test runs; refused, like a save, for a
    /// proofreading prompt without the placeholder.
    pub fn test_provider(&self, settings: &Settings) -> Result<(), SettingsError> {
        let mut control = self.shared.control();
        if let Err(refusal) = settings.check() {
            control.state.settings_error = Some(refusal.clone());
            drop(control);
            self.shared.notify();
            return Err(refusal);
        }
        if control.state.provider_test == Some(ProviderTest::Running) {
            return Ok(());
        }
        control.state.provider_test = Some(ProviderTest::Running);
        drop(control);
        self.shared.notify();
        let pending = provider::request_correction(settings, TEST_SENTENCE);
        let shared = Arc::clone(&self.shared);
        let spawned = thread::Builder::new().name("provider-test".to_owned()).spawn(move || {
            let outcome = match pending.wait() {
                Ok(correction) => ProviderTest::Corrected(correction),
                Err(error) => ProviderTest::Failed(error),
            };
            shared.change(|control| control.state.provider_test = Some(outcome));
        });
        if let Err(error) = spawned {
            self.shared.change(|control| control.state.provider_test = Some(ProviderTest::Failed(ProofreadError::Thread(error.to_string()))));
        }
        Ok(())
    }

    /// Takes a key event from the keyboard hook. When it completes a trigger and no proofread is
    /// running, a proofread starts on its own thread; a trigger during a proofread is ignored.
    /// Returns at once: the hook thread is never held.
    pub fn key_event(&self, event: KeyEvent) {
        self.shared.key_event(event);
    }

    /// The handle the keyboard hook of the platform holds, on a thread of its own.
    #[must_use]
    pub fn hook(&self) -> Hook {
        Hook { shared: Arc::clone(&self.shared) }
    }

    /// Cancel from the menu of the session icon, offered while a proofread runs
    /// ([`State::can_cancel`]). Without a running proofread, it does nothing.
    ///
    /// The proofread stops before the paste without waiting for the provider: nothing is pasted,
    /// the clipboard is given back, nothing is notified, and a correction arriving later is
    /// dropped. Once the paste has started, the proofread goes to its end.
    pub fn cancel(&self) {
        let running = self.shared.control().running.clone();
        if let Some(running) = running {
            // A proofread that ended meanwhile no longer listens: nothing left to cancel.
            let _ = running.send(Signal::Cancel);
        }
    }

    /// Waits until no proofread is running, for at most the given delay. True if so.
    pub fn wait_until_idle(&self, delay: Duration) -> bool {
        wait::wait_until(&self.shared.changed, self.shared.control(), delay, |control| control.state.activity == Activity::Idle)
    }
}

/// What the keyboard hook of the platform holds of the agent: it feeds it the key events and
/// reports whether it can listen. Cheap to clone, usable from any thread.
#[derive(Clone)]
pub struct Hook {
    shared: Arc<Shared>,
}

impl Hook {
    /// Takes a key event, as [`Agent::key_event`].
    pub fn key_event(&self, event: KeyEvent) {
        self.shared.key_event(event);
    }

    /// Reports the permission of the hook; the state shows it. Losing it asks for the window.
    pub fn permission(&self, permission: Permission) {
        let mut control = self.shared.control();
        if control.state.permission == permission {
            return;
        }
        control.state.permission = permission;
        if permission != Permission::Granted {
            control.state.window_requests = control.state.window_requests.saturating_add(1);
        }
        drop(control);
        self.shared.notify();
    }

    /// Reports that the hook cannot listen at all, for this cause: the state shows it, the window
    /// is asked for and the agent log keeps it.
    pub fn failed(&self, cause: &dyn Display) {
        let no_trigger = NoTrigger(cause.to_string());
        if let Some(log) = self.shared.log() {
            log.note(&no_trigger);
        }
        self.shared.change(|control| {
            control.state.no_trigger = Some(no_trigger);
            control.state.window_requests = control.state.window_requests.saturating_add(1);
        });
    }
}
