//! The agent: it receives the key events of the OS, detects the trigger and runs one proofread at
//! a time on a thread of its own, publishing its state as it goes.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use crate::error::ProofreadError;
use crate::os::{KeyEvent, Os};
use crate::proofread;
use crate::settings::Settings;
use crate::trigger::Detector;
use crate::wait;

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

/// What the agent shows of itself: the window and the icon render it as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// What the agent is doing.
    pub activity: Activity,
    /// The cause of the last failed proofread, cleared when a proofread starts.
    pub last_failure: Option<ProofreadError>,
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
    /// The OS, used by one proofread at a time.
    os: Mutex<Box<dyn Os>>,
    pub(crate) timing: Timing,
}

struct Control {
    state: State,
    settings: Settings,
    detector: Detector,
}

impl Shared {
    /// The control part, even if a thread panicked while holding it.
    fn control(&self) -> MutexGuard<'_, Control> {
        self.control.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The OS, even if a thread panicked while holding it.
    pub(crate) fn os(&self) -> MutexGuard<'_, Box<dyn Os>> {
        self.os.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Moves the running proofread to the given step.
    pub(crate) fn step(&self, step: Step) {
        self.control().state.activity = Activity::Proofreading(step);
        self.changed.notify_all();
    }

    /// Ends the running proofread, with its failure if it failed.
    fn end(&self, failure: Option<ProofreadError>) {
        let mut control = self.control();
        control.state.activity = Activity::Idle;
        control.state.last_failure = failure;
        drop(control);
        self.changed.notify_all();
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

/// The headless agent.
pub struct Agent {
    shared: Arc<Shared>,
}

impl Agent {
    /// An agent listening for the trigger of the settings, with the default timing.
    pub fn new(os: impl Os, settings: Settings) -> Self {
        Self::with_timing(os, settings, Timing::default())
    }

    /// An agent with the given delays around the copy and the paste.
    pub fn with_timing(os: impl Os, settings: Settings, timing: Timing) -> Self {
        let control = Control { state: State { activity: Activity::Idle, last_failure: None }, settings, detector: Detector::new() };
        Self { shared: Arc::new(Shared { control: Mutex::new(control), changed: Condvar::new(), os: Mutex::new(Box::new(os)), timing }) }
    }

    /// The current state.
    pub fn state(&self) -> State {
        self.shared.control().state.clone()
    }

    /// Takes a key event from the keyboard hook. When it completes a trigger and no proofread is
    /// running, a proofread starts on its own thread; a trigger during a proofread is ignored.
    /// Returns at once: the hook thread is never held.
    pub fn key_event(&self, event: KeyEvent) {
        let mut control = self.shared.control();
        let trigger = control.settings.trigger;
        if !control.detector.feed(&event, trigger) || control.state.activity != Activity::Idle {
            return;
        }
        // The guard is taken here, on the trigger: a second trigger during the capture is ignored.
        control.state.activity = Activity::Proofreading(Step::Capturing);
        control.state.last_failure = None;
        let settings = control.settings.clone();
        drop(control);
        self.shared.changed.notify_all();
        let shared = Arc::clone(&self.shared);
        let spawned = thread::Builder::new().name("proofread".to_owned()).spawn(move || {
            let mut ending = Ending { shared: Arc::clone(&shared), failure: None };
            ending.failure = proofread::run(&shared, &settings).err();
        });
        if let Err(error) = spawned {
            self.shared.end(Some(ProofreadError::Thread(error.to_string())));
        }
    }

    /// Waits until no proofread is running, for at most the given delay. True if so.
    pub fn wait_until_idle(&self, delay: Duration) -> bool {
        wait::wait_until(&self.shared.changed, self.shared.control(), delay, |control| control.state.activity == Activity::Idle)
    }
}
