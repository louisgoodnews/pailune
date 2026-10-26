//! Choosing the engine for a text patch file by its target: CSS, XML or
//! HTML (`docs/paz-format.md` §10.5).

use super::xml::Outcome;
use crate::error::{PyluneError, Result};

/// Result of a text patch: new text, a log line, and whether every target
/// of the patch was found.
pub type TextResult = (String, String, bool);

/// Summary line of an XML or HTML outcome.
fn summary(out: Outcome, kind: &str) -> TextResult {
    let missed = match out.missed.len() {
        0 => String::new(),
        n => format!("; {n} target(s) not found: {}", out.missed.join(", ")),
    };
    let line = format!("{kind} patch applied ({} change(s)){missed}", out.applied);
    (out.text, line, out.missed.is_empty())
}

/// Applies the patch file `file_name` (contents `patch`) to `base`, the
/// current text of `target`. Returns the new text and a log line.
pub fn apply(target: &str, file_name: &str, base: &str, patch: &str) -> Result<TextResult> {
    let ext = target.rsplit('.').next().unwrap_or_default();
    match ext {
        "xml" => super::xml::apply(base, file_name, patch)
            .map(|out| summary(out, "XML"))
            .map_err(PyluneError::Invalid),
        "html" => Ok(summary(super::html::apply(base, patch), "HTML")),
        _ => Ok((
            super::css::apply(base, patch),
            "CSS patch applied".into(),
            true,
        )),
    }
}
