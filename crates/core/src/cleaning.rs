//! The cleaning of the text returned by the provider into the correction.

use crate::error::ProofreadError;

const OPENING: &str = "<texte>";
const CLOSING: &str = "</texte>";

/// Turns the text returned by the provider into the correction of the selection: strips a
/// `<texte>…</texte>` wrapping the whole text, then puts back the leading and trailing whitespace
/// of the selection. Code fences are text like any other and stay. Empty is a failure.
pub(crate) fn clean(returned: &str, selection: &str) -> Result<String, ProofreadError> {
    let mut core = returned.trim();
    if let Some(inner) = core.strip_prefix(OPENING).and_then(|rest| rest.strip_suffix(CLOSING)) {
        core = inner.trim();
    }
    if core.is_empty() {
        return Err(ProofreadError::EmptyCorrection);
    }
    let trimmed_start = selection.trim_start();
    let leading = &selection[..selection.len() - trimmed_start.len()];
    let trailing = &trimmed_start[trimmed_start.trim_end().len()..];
    Ok(format!("{leading}{core}{trailing}"))
}
