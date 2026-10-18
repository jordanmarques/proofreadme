//! The proofread flow (ADR 0002), run on the thread of a proofread: save the clipboard, copy the
//! selection, request the correction, paste it, restore the clipboard.

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

/// Runs one proofread. Whatever happens after the save, a clipboard changed since then is given
/// back.
pub(crate) fn run(shared: &Shared, settings: &Settings) -> Result<(), ProofreadError> {
    let (saved, before) = {
        let mut os = shared.os();
        (os.read_clipboard()?, os.clipboard_change_count())
    };
    let replaced = replace_selection(shared, settings, before);
    let mut os = shared.os();
    if os.clipboard_change_count() == before {
        return replaced;
    }
    let restored = os.write_clipboard(&saved, Mark::Transient).map_err(ProofreadError::from);
    replaced.and(restored)
}

/// Copies the selection, then puts its correction in place. Nothing selected ends here, silently.
fn replace_selection(shared: &Shared, settings: &Settings, before: u64) -> Result<(), ProofreadError> {
    shared.os().synthesize(Keystroke::Copy)?;
    let Some(selection) = copied_selection(shared, before)? else {
        return Ok(());
    };
    shared.step(Step::Requesting);
    let correction = provider::request_correction(settings, &selection).wait()?;
    if correction == selection {
        return Ok(());
    }
    shared.step(Step::Replacing);
    let mut os = shared.os();
    os.write_clipboard(&ClipboardContent::text(correction), Mark::Transient)?;
    os.synthesize(Keystroke::Paste)?;
    drop(os);
    // The application reads the clipboard after the paste, in its own time: no signal says when.
    thread::sleep(shared.timing.paste_settle);
    Ok(())
}

/// The selection, once the application has copied it: `None` if the clipboard did not change in
/// time (nothing selected, or an application that refuses the keystroke), or holds no text.
fn copied_selection(shared: &Shared, before: u64) -> Result<Option<String>, ProofreadError> {
    let deadline = wait::deadline(shared.timing.copy_wait);
    loop {
        let mut os = shared.os();
        if os.clipboard_change_count() != before {
            let text = os.read_clipboard()?.text.unwrap_or_default();
            return Ok((!text.trim().is_empty()).then_some(text));
        }
        drop(os);
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        thread::sleep(remaining.min(COPY_POLL));
    }
}
