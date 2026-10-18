//! The proofread flow (ADR 0002), run on the thread of a proofread: save the clipboard, copy the
//! selection, request the correction, paste it, restore the clipboard.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::agent::{Shared, Step};
use crate::error::ProofreadError;
use crate::os::{ClipboardContent, Keystroke, Mark};
use crate::provider;
use crate::settings::Settings;
use crate::wait;

/// The interval at which the clipboard is checked while waiting for the copy.
const COPY_POLL: Duration = Duration::from_millis(5);

/// What reaches a running proofread from the other threads.
pub(crate) enum Signal {
    /// The correction of the provider, or why there is none.
    Correction(Result<String, ProofreadError>),
    /// Cancel from the menu of the session icon.
    Cancel,
}

/// How a proofread that did not fail ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The selection was replaced, or there was nothing to replace.
    Finished,
    /// Cancelled before the paste.
    Cancelled,
    /// The foreground application changed during the request: the correction is on the clipboard.
    LeftOnClipboard,
}

/// Runs one proofread. Whatever happens after the save, a clipboard changed since then is given
/// back, except when the correction is left on it on purpose.
///
/// `signals` receives the cancel; `sender` is its other end, handed to the provider.
pub(crate) fn run(shared: &Shared, settings: &Settings, signals: &Receiver<Signal>, sender: &Sender<Signal>) -> Result<Outcome, ProofreadError> {
    let (saved, before, foreground) = {
        let mut os = shared.os();
        (os.read_clipboard()?, os.clipboard_change_count(), os.foreground_application())
    };
    let replaced = replace_selection(shared, settings, Exchange { signals, sender, before, foreground });
    if replaced == Ok(Outcome::LeftOnClipboard) {
        return replaced;
    }
    let mut os = shared.os();
    if os.clipboard_change_count() == before {
        return replaced;
    }
    let restored = os.write_clipboard(&saved, Mark::Transient).map_err(ProofreadError::from);
    replaced.and_then(|outcome| restored.map(|()| outcome))
}

/// What the proofread knows of its start, and its channels.
struct Exchange<'a> {
    signals: &'a Receiver<Signal>,
    sender: &'a Sender<Signal>,
    /// The change count of the clipboard before the copy.
    before: u64,
    /// The application the selection is read from.
    foreground: Option<String>,
}

/// Copies the selection, then puts its correction in place. Nothing selected ends here, silently.
fn replace_selection(shared: &Shared, settings: &Settings, exchange: Exchange<'_>) -> Result<Outcome, ProofreadError> {
    shared.os().synthesize(Keystroke::Copy)?;
    let selection = match copied_selection(shared, &exchange)? {
        Copied::Selection(selection) => selection,
        Copied::Nothing => return Ok(Outcome::Finished),
        Copied::Cancelled => return Ok(Outcome::Cancelled),
    };
    shared.step(Step::Requesting);
    provider::send_to(settings, &selection, exchange.sender, Signal::Correction);
    let correction = match wait_for_correction(exchange.signals, settings.timeout) {
        Signal::Correction(correction) => correction?,
        Signal::Cancel => return Ok(Outcome::Cancelled),
    };
    if correction == selection {
        return Ok(Outcome::Finished);
    }
    shared.step(Step::Replacing);
    let mut os = shared.os();
    // The correction lands where the selection was, or nowhere (ADR 0002).
    if os.foreground_application() != exchange.foreground {
        os.write_clipboard(&ClipboardContent::text(correction), Mark::Kept)?;
        return Ok(Outcome::LeftOnClipboard);
    }
    os.write_clipboard(&ClipboardContent::text(correction), Mark::Transient)?;
    os.synthesize(Keystroke::Paste)?;
    drop(os);
    // The application reads the clipboard after the paste, in its own time: no signal says when.
    thread::sleep(shared.timing.paste_settle);
    Ok(Outcome::Finished)
}

/// The correction or the cancel, whichever comes first. The provider gives up at the timeout;
/// should its thread still not answer, the wait gives up a little later.
fn wait_for_correction(signals: &Receiver<Signal>, timeout: Duration) -> Signal {
    /// What the wait grants the provider beyond the timeout, to report its own cause.
    const GRACE: Duration = Duration::from_secs(5);

    let deadline = wait::deadline(timeout.saturating_add(GRACE));
    match signals.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(signal) => signal,
        Err(RecvTimeoutError::Timeout) => Signal::Correction(Err(ProofreadError::Timeout)),
        Err(RecvTimeoutError::Disconnected) => {
            Signal::Correction(Err(ProofreadError::Thread("the request thread ended without an answer".to_owned())))
        }
    }
}

/// What the copy brought.
enum Copied {
    /// The selected text.
    Selection(String),
    /// Nothing selected (or an application that refuses the keystroke), or no text.
    Nothing,
    /// Cancelled while waiting for the copy.
    Cancelled,
}

/// The selection, once the application has copied it, unless the clipboard did not change in
/// time or the proofread is cancelled meanwhile.
fn copied_selection(shared: &Shared, exchange: &Exchange<'_>) -> Result<Copied, ProofreadError> {
    let deadline = wait::deadline(shared.timing.copy_wait);
    loop {
        if matches!(exchange.signals.try_recv(), Ok(Signal::Cancel) | Err(TryRecvError::Disconnected)) {
            return Ok(Copied::Cancelled);
        }
        let mut os = shared.os();
        if os.clipboard_change_count() != exchange.before {
            let text = os.read_clipboard()?.text.unwrap_or_default();
            return Ok(if text.trim().is_empty() { Copied::Nothing } else { Copied::Selection(text) });
        }
        drop(os);
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(Copied::Nothing);
        }
        thread::sleep(remaining.min(COPY_POLL));
    }
}
