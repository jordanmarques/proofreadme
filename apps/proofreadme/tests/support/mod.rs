//! What the window tests share: an agent over a simulated data folder and an in-memory keychain,
//! its wakeups, and a real local HTTP server standing in for the provider.
#![allow(dead_code, reason = "each test file uses part of the support")]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use proofreadme::{UnsupportedOs, Window};
use proofreadme_core::{Agent, MemoryKeychain, Provider, Storage, Timing};

/// The delay beyond which a test gives up waiting: never reached when all goes well.
pub const DELAY: Duration = Duration::from_secs(20);

/// The API key of the tests.
pub const API_KEY: &str = "sk-window-key";

/// The location of the config in a data folder simulated in `folder`.
pub fn config(folder: &Path) -> PathBuf {
    folder.join("ProofReadMe").join("config.toml")
}

/// A provider at this base URL, with the key of the tests.
pub fn provider(base_url: &str) -> Provider {
    Provider { base_url: base_url.to_owned(), api_key: API_KEY.to_owned(), model: "vendor/model".to_owned() }
}

/// An agent over the data folder simulated in `folder`.
pub fn loaded(folder: &Path, keychain: &MemoryKeychain) -> Agent {
    Agent::load(UnsupportedOs, Storage::new(config(folder), keychain.clone()), Timing::default())
}

/// The agent, with its changes of state signalled on the returned channel.
pub fn observed(agent: Agent) -> (Agent, Receiver<()>) {
    let (sender, wakeups) = mpsc::channel();
    let agent = agent.on_change(move || {
        let _ = sender.send(());
    });
    (agent, wakeups)
}

/// Redraws the window at each wakeup of the agent until `shown` holds, for at most `DELAY`.
pub fn show_until(window: &mut Harness<'_, Window>, wakeups: &Receiver<()>, shown: impl Fn(&Harness<'_, Window>) -> bool) {
    let deadline = Instant::now() + DELAY;
    window.run_ok();
    while !shown(window) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "the window did not show the expected state in time");
        let _ = wakeups.recv_timeout(remaining);
        window.run_ok();
    }
}

/// What the local provider responds to the next request.
pub struct Reply {
    status: u16,
    body: String,
    hold: Option<Receiver<()>>,
}

impl Reply {
    /// A successful response whose first choice carries the given content.
    pub fn correction(content: &str) -> Self {
        let body = serde_json::json!({ "choices": [{ "message": { "role": "assistant", "content": content } }] });
        Self { status: 200, body: body.to_string(), hold: None }
    }

    /// An error status.
    pub fn status(status: u16) -> Self {
        Self { status, body: "{}".to_owned(), hold: None }
    }

    /// The same response, held until the returned sender signals (or is dropped).
    pub fn held(self) -> (Self, Sender<()>) {
        let (release, hold) = mpsc::channel();
        (Self { hold: Some(hold), ..self }, release)
    }
}

/// A real HTTP server on the local loop, standing in for the provider. Each request received is
/// signalled with the content of its message.
pub struct LocalProvider {
    address: String,
    replies: Arc<Mutex<VecDeque<Reply>>>,
    received: Receiver<String>,
}

impl LocalProvider {
    pub fn start() -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr().to_ip().unwrap());
        let replies: Arc<Mutex<VecDeque<Reply>>> = Arc::default();
        let (sender, received) = mpsc::channel();
        let queue = Arc::clone(&replies);
        thread::spawn(move || {
            for mut request in server.incoming_requests() {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).unwrap();
                let body: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
                let _ = sender.send(body["messages"][0]["content"].as_str().unwrap_or_default().to_owned());
                let reply = queue.lock().unwrap().pop_front().unwrap_or_else(|| Reply::correction("unscripted"));
                thread::spawn(move || {
                    if let Some(hold) = reply.hold {
                        let _ = hold.recv();
                    }
                    let _ = request.respond(tiny_http::Response::from_string(reply.body).with_status_code(reply.status));
                });
            }
        });
        Self { address, replies, received }
    }

    pub fn reply(&self, reply: Reply) {
        self.replies.lock().unwrap().push_back(reply);
    }

    pub fn base_url(&self) -> String {
        format!("{}/api/v1", self.address)
    }

    /// The contents of the requests received so far.
    pub fn received(&self) -> Vec<String> {
        self.received.try_iter().collect()
    }

    /// Waits for the next request, for at most `DELAY`.
    pub fn next_request(&self) -> String {
        self.received.recv_timeout(DELAY).expect("the provider received no request in time")
    }
}
