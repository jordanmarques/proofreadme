//! What the agent runs with: the trigger key, the provider and the proofreading prompt.

use std::fmt;
use std::time::Duration;

use crate::os::Modifier;

/// The placeholder of the proofreading prompt replaced by the selection.
pub const PLACEHOLDER: &str = "{{TEXTE}}";

/// The chat completions endpoint that produces the correction.
#[derive(Clone, PartialEq, Eq)]
pub struct Provider {
    /// The base URL, `/chat/completions` is appended to it.
    pub base_url: String,
    /// The API key, sent as a bearer token. Never printed.
    pub api_key: String,
    /// The model.
    pub model: String,
}

impl fmt::Debug for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Provider").field("base_url", &self.base_url).field("api_key", &"<hidden>").field("model", &self.model).finish()
    }
}

/// The settings a proofread runs with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The modifier whose double press is the trigger.
    pub trigger: Modifier,
    /// The provider.
    pub provider: Provider,
    /// The proofreading prompt, carrying the `{{TEXTE}}` placeholder.
    pub prompt: String,
    /// Sends `"reasoning": {"enabled": true}` when on.
    pub reasoning: bool,
    /// The longest a request to the provider may take, answer included.
    pub timeout: Duration,
}
