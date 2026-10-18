//! The client of the provider: one chat completion per proofread, sent over blocking HTTP on its
//! own thread so that the caller (the thread of a proofread, or the window for the Test button)
//! never waits on the network.

use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde::Deserialize;

use crate::cleaning;
use crate::error::ProofreadError;
use crate::settings::{PLACEHOLDER, Settings};

/// The longest timeout actually applied: a larger one from the settings is brought down to it.
const MAX_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// A request sent to the provider, whose correction arrives later.
#[derive(Debug)]
pub struct PendingCorrection {
    receiver: Receiver<Result<String, ProofreadError>>,
}

impl PendingCorrection {
    /// Waits for the correction, cleaned. The wait is bounded by the timeout of the settings.
    pub fn wait(self) -> Result<String, ProofreadError> {
        // The thread always sends before ending, unless it panicked.
        self.receiver.recv().unwrap_or_else(|_| Err(ProofreadError::Thread("the request thread ended without an answer".to_owned())))
    }

    /// The correction if it has arrived, without waiting.
    pub fn try_take(&self) -> Option<Result<String, ProofreadError>> {
        self.receiver.try_recv().ok()
    }
}

/// Sends the selection inside the proofreading prompt to the provider, on a thread of its own.
pub fn request_correction(settings: &Settings, selection: &str) -> PendingCorrection {
    let (sender, receiver) = mpsc::channel();
    let settings = settings.clone();
    let selection = selection.to_owned();
    let spawned = thread::Builder::new().name("provider".to_owned()).spawn({
        let sender = sender.clone();
        move || {
            let _ = sender.send(correct(&settings, &selection));
        }
    });
    if let Err(error) = spawned {
        let _ = sender.send(Err(ProofreadError::Thread(error.to_string())));
    }
    PendingCorrection { receiver }
}

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

fn correct(settings: &Settings, selection: &str) -> Result<String, ProofreadError> {
    let returned = send(settings, selection)?;
    cleaning::clean(&returned, selection)
}

/// The text of the first choice returned by the provider.
fn send(settings: &Settings, selection: &str) -> Result<String, ProofreadError> {
    let provider = &settings.provider;
    let url = format!("{}/chat/completions", provider.base_url.trim_end_matches('/'));
    let mut body = serde_json::json!({
        "model": provider.model,
        "messages": [{ "role": "user", "content": settings.prompt.replace(PLACEHOLDER, selection) }],
    });
    if settings.reasoning {
        body["reasoning"] = serde_json::json!({ "enabled": true });
    }
    let agent: ureq::Agent =
        ureq::Agent::config_builder().timeout_global(Some(settings.timeout.min(MAX_TIMEOUT))).http_status_as_error(false).build().into();
    let mut response = agent
        .post(&url)
        .header("Authorization", &format!("Bearer {}", provider.api_key))
        .content_type("application/json")
        .send(body.to_string())
        .map_err(failure)?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(ProofreadError::Status(status));
    }
    let text = response.body_mut().read_to_string().map_err(failure)?;
    let completion: Completion = serde_json::from_str(&text).map_err(|_| ProofreadError::MalformedAnswer)?;
    completion.choices.into_iter().next().and_then(|choice| choice.message.content).ok_or(ProofreadError::MalformedAnswer)
}

/// The cause of a failed exchange. The message of ureq names the URL and the transport error,
/// never the body.
fn failure(error: ureq::Error) -> ProofreadError {
    match error {
        ureq::Error::Timeout(_) => ProofreadError::Timeout,
        other => ProofreadError::Network(other.to_string()),
    }
}
