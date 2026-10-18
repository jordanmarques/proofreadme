//! What the tests of the headless agent share: a fake OS port and a real local HTTP server
//! standing in for the provider.
#![allow(dead_code, reason = "each test file uses part of the support")]

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use proofreadme_core::{
    Agent, ClipboardContent, Direction, Image, Key, KeyEvent, Keystroke, Mark, Modifier, Notification, Notifier, Os, OsError, Provider, Settings,
    Timing,
};

/// The delay beyond which a test gives up waiting for the agent: never reached when all goes well.
pub const DELAY: Duration = Duration::from_secs(20);

/// The API key of the tests.
pub const API_KEY: &str = "sk-test-key";

/// The model of the tests.
pub const MODEL: &str = "vendor/model:free";

// ---------------------------------------------------------------------------------------------
// Fake OS port
// ---------------------------------------------------------------------------------------------

/// A simulated desktop: a clipboard that records its marks, a foreground application holding a
/// selection, and the keystrokes synthesized by the agent.
#[derive(Clone, Default)]
pub struct Desktop {
    inner: Arc<Mutex<DesktopState>>,
}

#[derive(Default)]
struct DesktopState {
    clipboard: ClipboardContent,
    /// The mark of the last write of the clipboard, `None` when written by the user.
    mark: Option<Mark>,
    change_count: u64,
    writes: Vec<(ClipboardContent, Mark)>,
    selection: Option<String>,
    keystrokes: Vec<Keystroke>,
    pasted: Vec<String>,
    foreground: String,
}

impl Desktop {
    pub fn new() -> Self {
        let desktop = Self::default();
        desktop.state().foreground = "editor".to_owned();
        desktop
    }

    fn state(&self) -> std::sync::MutexGuard<'_, DesktopState> {
        self.inner.lock().unwrap()
    }

    /// The user selects text in the foreground application.
    pub fn select(&self, text: &str) {
        self.state().selection = Some(text.to_owned());
    }

    /// The user copies something themselves.
    pub fn user_copies(&self, content: ClipboardContent) {
        let mut state = self.state();
        state.clipboard = content;
        state.mark = None;
        state.change_count += 1;
    }

    pub fn clipboard(&self) -> ClipboardContent {
        self.state().clipboard.clone()
    }

    pub fn writes(&self) -> Vec<(ClipboardContent, Mark)> {
        self.state().writes.clone()
    }

    pub fn keystrokes(&self) -> Vec<Keystroke> {
        self.state().keystrokes.clone()
    }

    /// The text of the clipboard at each synthesized paste: what landed in the application.
    pub fn pasted(&self) -> Vec<String> {
        self.state().pasted.clone()
    }

    /// The text selected in the application, replaced by each paste.
    pub fn selection(&self) -> Option<String> {
        self.state().selection.clone()
    }

    /// The user brings another application to the foreground.
    pub fn focus(&self, application: &str) {
        self.state().foreground = application.to_owned();
    }

    /// The mark of the last write of the clipboard, `None` when written by the user.
    pub fn mark(&self) -> Option<Mark> {
        self.state().mark
    }
}

impl Os for Desktop {
    fn read_clipboard(&mut self) -> Result<ClipboardContent, OsError> {
        Ok(self.state().clipboard.clone())
    }

    fn write_clipboard(&mut self, content: &ClipboardContent, mark: Mark) -> Result<(), OsError> {
        let mut state = self.state();
        state.clipboard = content.clone();
        state.mark = Some(mark);
        state.change_count += 1;
        state.writes.push((content.clone(), mark));
        Ok(())
    }

    fn clipboard_change_count(&mut self) -> u64 {
        self.state().change_count
    }

    fn synthesize(&mut self, keystroke: Keystroke) -> Result<(), OsError> {
        let mut state = self.state();
        state.keystrokes.push(keystroke);
        match keystroke {
            Keystroke::Copy => {
                // An application copies nothing when nothing is selected.
                if let Some(selection) = state.selection.clone() {
                    state.clipboard = ClipboardContent::text(selection);
                    state.mark = None;
                    state.change_count += 1;
                }
            }
            Keystroke::Paste => {
                let text = state.clipboard.text.clone().unwrap_or_default();
                state.pasted.push(text.clone());
                state.selection = Some(text);
            }
        }
        Ok(())
    }

    fn foreground_application(&mut self) -> Option<String> {
        Some(self.state().foreground.clone())
    }
}

/// A small image, as a user would have copied it.
pub fn image() -> Image {
    Image { width: 2, height: 1, rgba: vec![255, 0, 0, 255, 0, 0, 255, 255] }
}

// ---------------------------------------------------------------------------------------------
// Local provider
// ---------------------------------------------------------------------------------------------

/// A request received by the local provider.
#[derive(Debug, Clone)]
pub struct Received {
    pub method: String,
    pub url: String,
    pub authorization: Option<String>,
    pub content_type: Option<String>,
    pub body: serde_json::Value,
}

/// What the local provider answers to the next request.
pub struct Reply {
    status: u16,
    body: String,
    /// When set, the answer waits for a signal on this channel.
    hold: Option<Receiver<()>>,
}

impl Reply {
    /// A successful answer whose first choice carries the given content.
    pub fn correction(content: &str) -> Self {
        let body = serde_json::json!({
            "id": "gen-1",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": content }, "finish_reason": "stop" }],
        });
        Self { status: 200, body: body.to_string(), hold: None }
    }

    /// An error status, with an error body.
    pub fn status(status: u16) -> Self {
        Self { status, body: r#"{"error":{"message":"refused"}}"#.to_owned(), hold: None }
    }

    /// The same answer, held until the returned sender signals (or is dropped).
    pub fn held(self) -> (Self, Sender<()>) {
        let (release, hold) = mpsc::channel();
        (Self { hold: Some(hold), ..self }, release)
    }
}

/// A real HTTP server on the local loop, standing in for the provider.
pub struct LocalProvider {
    address: String,
    replies: Arc<Mutex<VecDeque<Reply>>>,
    received: Receiver<Received>,
}

impl LocalProvider {
    pub fn start() -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr().to_ip().unwrap());
        let replies: Arc<Mutex<VecDeque<Reply>>> = Arc::default();
        let (sender, received) = mpsc::channel();
        let queue = Arc::clone(&replies);
        thread::Builder::new()
            .name("local-provider".to_owned())
            .spawn(move || {
                for mut request in server.incoming_requests() {
                    let mut body = String::new();
                    request.as_reader().read_to_string(&mut body).unwrap();
                    let header = |name: &'static str| request.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str().to_owned());
                    let received = Received {
                        method: request.method().as_str().to_owned(),
                        url: request.url().to_owned(),
                        authorization: header("Authorization"),
                        content_type: header("Content-Type"),
                        body: serde_json::from_str(&body).unwrap_or(serde_json::Value::Null),
                    };
                    let _ = sender.send(received);
                    let reply = queue.lock().unwrap().pop_front().unwrap_or_else(|| Reply::correction("unscripted"));
                    // Answered on its own thread: a held answer does not block the next request.
                    thread::spawn(move || {
                        if let Some(hold) = reply.hold {
                            let _ = hold.recv();
                        }
                        let response = tiny_http::Response::from_string(reply.body)
                            .with_status_code(reply.status)
                            .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap());
                        let _ = request.respond(response);
                    });
                }
            })
            .unwrap();
        Self { address, replies, received }
    }

    /// Scripts the answer to the next request.
    pub fn reply(&self, reply: Reply) {
        self.replies.lock().unwrap().push_back(reply);
    }

    /// The base URL of the provider, as the user would set it.
    pub fn base_url(&self) -> String {
        format!("{}/api/v1", self.address)
    }

    /// The requests received so far.
    pub fn received(&self) -> Vec<Received> {
        self.received.try_iter().collect()
    }

    /// Waits for the next request, for at most `DELAY`.
    pub fn next_request(&self) -> Received {
        self.received.recv_timeout(DELAY).expect("the provider received no request in time")
    }
}

// ---------------------------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------------------------

/// The proofreading prompt of the tests.
pub const PROMPT: &str = "Corrige :\n<texte>\n{{TEXTE}}\n</texte>";

/// The settings of the tests, pointing at the local provider.
pub fn settings(provider: &LocalProvider) -> Settings {
    Settings {
        trigger: Modifier::Ctrl,
        provider: Provider { base_url: provider.base_url(), api_key: API_KEY.to_owned(), model: MODEL.to_owned() },
        prompt: PROMPT.to_owned(),
        reasoning: true,
        timeout: DELAY,
        launch_at_login: false,
    }
}

/// The timing of the tests: the fake applications answer at once, so the agent waits little for
/// a copy that brings nothing, and does not wait before restoring the clipboard.
pub fn timing() -> Timing {
    Timing { copy_wait: Duration::from_millis(200), paste_settle: Duration::ZERO }
}

pub fn agent(desktop: &Desktop, settings: Settings) -> Agent {
    Agent::with_timing(desktop.clone(), settings, timing())
}

/// Feeds key events to the agent, each `gap` after the previous one, starting now.
pub struct Keyboard {
    now: Instant,
}

impl Keyboard {
    pub fn new() -> Self {
        Self { now: Instant::now() }
    }

    /// Moves the clock of the keyboard forward.
    pub fn wait(&mut self, delay: Duration) {
        self.now += delay;
    }

    pub fn down(&mut self, agent: &Agent, key: Key) {
        agent.key_event(KeyEvent { key, direction: Direction::Down, at: self.now, injected: false });
    }

    pub fn up(&mut self, agent: &Agent, key: Key) {
        agent.key_event(KeyEvent { key, direction: Direction::Up, at: self.now, injected: false });
    }

    /// Down then up, 40 ms apart, then 40 ms of rest.
    pub fn tap(&mut self, agent: &Agent, key: Key) {
        self.down(agent, key);
        self.wait(Duration::from_millis(40));
        self.up(agent, key);
        self.wait(Duration::from_millis(40));
    }

    /// The trigger: two quick taps of the modifier.
    pub fn double_press(&mut self, agent: &Agent, modifier: Modifier) {
        self.tap(agent, Key::Modifier(modifier));
        self.tap(agent, Key::Modifier(modifier));
    }
}

/// A notification sink that records what the agent notifies.
#[derive(Clone, Default)]
pub struct Notifications {
    sent: Arc<Mutex<Vec<Notification>>>,
}

impl Notifications {
    pub fn sent(&self) -> Vec<Notification> {
        self.sent.lock().unwrap().clone()
    }
}

impl Notifier for Notifications {
    fn notify(&self, notification: &Notification) {
        self.sent.lock().unwrap().push(notification.clone());
    }
}

/// What an agent tells outside: its notifications and its log, in a folder of its own.
pub struct Outside {
    pub notifications: Notifications,
    folder: tempfile::TempDir,
}

impl Outside {
    pub fn new() -> Self {
        Self { notifications: Notifications::default(), folder: tempfile::tempdir().unwrap() }
    }

    /// Where the agent writes its log.
    pub fn log_path(&self) -> std::path::PathBuf {
        self.folder.path().join("logs").join("agent.txt")
    }

    /// The agent log, empty if never written.
    pub fn log(&self) -> String {
        std::fs::read_to_string(self.log_path()).unwrap_or_default()
    }

    pub fn sent(&self) -> Vec<Notification> {
        self.notifications.sent()
    }
}

/// An agent whose notifications and log land in `outside`.
pub fn watched_agent(desktop: &Desktop, settings: Settings, outside: &Outside) -> Agent {
    agent(desktop, settings).with_notifier(outside.notifications.clone()).with_log(outside.log_path())
}

/// Waits until the agent has no proofread running.
pub fn wait_idle(agent: &Agent) {
    assert!(agent.wait_until_idle(DELAY), "the proofread did not end in time");
}
