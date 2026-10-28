//! Applying a file patch to plaintext game data.

use super::table::TableIndex;
use super::{Anchor, ApplyReport, Change, Edit, FilePatch};
use std::collections::HashMap;

/// A change resolved to an absolute offset.
struct Resolved<'a> {
    at: usize,
    change: &'a Change,
}

/// Label of a change for reports (its label, else its offset).
fn describe(change: &Change, at: Option<usize>) -> String {
    match (change.label.is_empty(), at) {
        (false, _) => change.label.clone(),
        (true, Some(at)) => format!("offset {at:#x}"),
        (true, None) => "change".into(),
    }
}

/// Finds the first occurrence of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len().max(1))
        .position(|w| w == needle)
}

/// Where a patch's changes start: the signature's end (if any) and the
/// table's record anchors. `None` when the signature is missing.
pub(crate) struct Frame {
    pub(crate) base: usize,
    pub(crate) anchors: Option<HashMap<String, usize>>,
}

impl Frame {
    pub(crate) fn of(data: &[u8], patch: &FilePatch, header: Option<&[u8]>) -> Option<Self> {
        let base = match &patch.signature {
            Some(sig) => find(data, sig)? + sig.len(),
            None => 0,
        };
        let anchors = header
            .and_then(super::table::parse_index)
            .map(|i: TableIndex| i.name_anchors(data));
        Some(Self { base, anchors })
    }

    /// Absolute offset of a change.
    pub(crate) fn position(&self, change: &Change) -> Option<usize> {
        match &change.anchor {
            Anchor::Offset(o) => usize::try_from(*o)
                .ok()
                .and_then(|o| self.base.checked_add(o)),
            Anchor::Entry { name, rel_offset } => self
                .anchors
                .as_ref()
                .and_then(|a| a.get(name))
                .and_then(|a| {
                    usize::try_from(*rel_offset)
                        .ok()
                        .and_then(|r| a.checked_add(r))
                }),
        }
    }
}

/// Byte ranges `(start, end, label)` the replacements of `patch` cover in
/// `data` (inserts and unresolvable changes are left out).
pub fn spans(data: &[u8], patch: &FilePatch, header: Option<&[u8]>) -> Vec<(usize, usize, String)> {
    let Some(frame) = Frame::of(data, patch, header) else {
        return Vec::new();
    };
    patch
        .changes
        .iter()
        .filter_map(|c| match &c.edit {
            Edit::Replace { original, patched } => frame.position(c).map(|at| {
                let len = original.as_ref().map_or(patched.len(), Vec::len);
                (at, at + len, describe(c, Some(at)))
            }),
            Edit::Insert { .. } => None,
        })
        .collect()
}

/// Applies `patch` to `data`. `header` is the table index for entry
/// anchors; `is_table` forbids inserts (they would break the index).
/// Without `allow_partial`, any failing change leaves `data` untouched.
#[cfg(test)]
pub fn apply(
    data: &mut Vec<u8>,
    patch: &FilePatch,
    header: Option<&[u8]>,
    is_table: bool,
    allow_partial: bool,
) -> ApplyReport {
    apply_over(data, patch, header, is_table, allow_partial, None)
}

/// [`apply`] on data that earlier mods may have changed: an `original` that
/// matches `reference` (the unmodded bytes, same length) also counts, so a
/// later mod overrides an earlier one instead of failing — later mods win.
pub fn apply_over(
    data: &mut Vec<u8>,
    patch: &FilePatch,
    header: Option<&[u8]>,
    is_table: bool,
    allow_partial: bool,
    reference: Option<&[u8]>,
) -> ApplyReport {
    let Some(frame) = Frame::of(data, patch, header) else {
        return ApplyReport {
            failed: vec!["signature not found".into()],
            ..ApplyReport::default()
        };
    };
    // Positions match the reference only while no insert moved them.
    let reference = reference.filter(|r| r.len() == data.len());
    let mut report = ApplyReport::default();
    let mut valid: Vec<Resolved> = Vec::new();
    patch.changes.iter().for_each(|change| {
        let at = frame.position(change);
        let verdict = match (at, &change.edit) {
            (None, _) => Err("entry not found in the table".to_string()),
            (Some(_), Edit::Insert { .. }) if is_table => {
                Err("inserts into data tables are not supported".into())
            }
            (Some(at), Edit::Insert { .. }) if at > data.len() => {
                Err("offset beyond the end of the file".into())
            }
            (Some(_), Edit::Insert { .. }) => Ok(false),
            (
                Some(_),
                Edit::Replace {
                    original: Some(o),
                    patched,
                },
            ) if is_table && o.len() != patched.len() => {
                Err("changes to data tables must keep their length".into())
            }
            (Some(at), Edit::Replace { original, patched }) => {
                let span = original.as_ref().map_or(patched.len(), Vec::len);
                match at.checked_add(span).and_then(|end| data.get(at..end)) {
                    None => Err("offset beyond the end of the file".into()),
                    Some(now) if now == patched.as_slice() => Ok(true),
                    Some(now) if original.as_ref().is_none_or(|o| o.as_slice() == now) => Ok(false),
                    Some(_)
                        if original.as_ref().is_some_and(|o| {
                            reference.and_then(|r| r.get(at..at + span)) == Some(o.as_slice())
                        }) =>
                    {
                        Ok(false)
                    }
                    Some(_) => Err("original bytes do not match (different game version?)".into()),
                }
            }
        };
        match verdict {
            Ok(true) => report.already += 1,
            Ok(false) => valid.push(Resolved {
                at: at.unwrap_or(0),
                change,
            }),
            Err(reason) => report
                .failed
                .push(format!("{}: {reason}", describe(change, at))),
        }
    });
    // Two changes to the same bytes cannot both be honoured: the later one
    // (in file order) is refused instead of silently overwriting the first.
    let span = |r: &Resolved| match &r.change.edit {
        Edit::Insert { .. } => 0,
        Edit::Replace { original, patched } => original.as_ref().map_or(patched.len(), Vec::len),
    };
    let mut taken: Vec<(usize, usize)> = Vec::new();
    valid.retain(|r| {
        let (start, end) = (r.at, r.at + span(r));
        let clash = taken.iter().any(|(s, e)| match end == start {
            true => *s < start && start < *e,
            false => start < *e && *s < end,
        });
        match clash {
            true => {
                report.failed.push(format!(
                    "{}: overlaps an earlier change",
                    describe(r.change, Some(r.at))
                ));
                false
            }
            false => {
                taken.push((start, end));
                true
            }
        }
    });
    if !report.failed.is_empty() && !allow_partial {
        return ApplyReport {
            applied: 0,
            ..report
        };
    }
    // Highest offset first: size changes never move offsets still to apply.
    valid.sort_by_key(|r| std::cmp::Reverse(r.at));
    valid.iter().for_each(|r| match &r.change.edit {
        Edit::Insert { bytes } => drop(data.splice(r.at..r.at, bytes.iter().copied())),
        Edit::Replace { original, patched } => {
            let span = original.as_ref().map_or(patched.len(), Vec::len);
            drop(data.splice(r.at..r.at + span, patched.iter().copied()));
        }
    });
    report.applied = valid.len();
    report
}
