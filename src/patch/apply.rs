//! Applying a file patch to plaintext game data.

use super::table::TableIndex;
use super::{Anchor, ApplyReport, Change, Edit, FilePatch};

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

/// Applies `patch` to `data`. `header` is the table index for entry
/// anchors; `is_table` forbids inserts (they would break the index).
/// Without `allow_partial`, any failing change leaves `data` untouched.
pub fn apply(
    data: &mut Vec<u8>,
    patch: &FilePatch,
    header: Option<&[u8]>,
    is_table: bool,
    allow_partial: bool,
) -> ApplyReport {
    let base = match &patch.signature {
        Some(sig) => match find(data, sig) {
            Some(pos) => pos + sig.len(),
            None => {
                return ApplyReport {
                    failed: vec!["signature not found".into()],
                    ..ApplyReport::default()
                };
            }
        },
        None => 0,
    };
    let anchors = header
        .and_then(super::table::parse_index)
        .map(|i: TableIndex| i.name_anchors(data));
    let mut report = ApplyReport::default();
    let mut valid: Vec<Resolved> = Vec::new();
    patch.changes.iter().for_each(|change| {
        let at = match &change.anchor {
            Anchor::Offset(o) => usize::try_from(*o).ok().and_then(|o| base.checked_add(o)),
            Anchor::Entry { name, rel_offset } => {
                anchors.as_ref().and_then(|a| a.get(name)).and_then(|a| {
                    usize::try_from(*rel_offset)
                        .ok()
                        .and_then(|r| a.checked_add(r))
                })
            }
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::Edit;

    fn replace(at: u64, original: &[u8], patched: &[u8]) -> Change {
        Change {
            anchor: Anchor::Offset(at),
            edit: Edit::Replace {
                original: Some(original.to_vec()),
                patched: patched.to_vec(),
            },
            label: String::new(),
        }
    }

    #[test]
    fn verifies_originals_and_respects_partial_apply() {
        let patch = |changes| FilePatch {
            game_file: "x".into(),
            signature: None,
            changes,
            fields: Vec::new(),
        };
        let mut data = vec![0u8, 1, 2, 3, 4, 5];
        let good = patch(vec![replace(1, &[1, 2], &[9, 9]), replace(4, &[4], &[7])]);
        assert_eq!(apply(&mut data, &good, None, false, false).applied, 2);
        assert_eq!(data, [0, 9, 9, 3, 7, 5]);
        assert_eq!(
            apply(&mut data, &good, None, false, false).already,
            2,
            "idempotent"
        );
        let bad = patch(vec![replace(0, &[0], &[5]), replace(3, &[42], &[1])]);
        let strict = apply(&mut data, &bad, None, false, false);
        assert_eq!(
            (strict.applied, strict.failed.len(), data[0]),
            (0, 1, 0),
            "nothing applied"
        );
        assert_eq!(apply(&mut data, &bad, None, false, true).applied, 1);
        assert_eq!(data[0], 5);
        let insert = patch(vec![Change {
            anchor: Anchor::Offset(1),
            edit: Edit::Insert { bytes: vec![0xAA] },
            label: "ins".into(),
        }]);
        assert_eq!(
            apply(&mut data, &insert, None, true, false).failed.len(),
            1,
            "no inserts into tables"
        );
        apply(&mut data, &insert, None, false, false);
        assert_eq!(data[..3], [5, 0xAA, 9]);
    }
}

#[cfg(test)]
mod safety_tests {
    use super::*;
    use crate::patch::Edit;

    fn patch(changes: Vec<Change>) -> FilePatch {
        FilePatch {
            game_file: "x".into(),
            signature: None,
            changes,
            fields: Vec::new(),
        }
    }

    fn replace(at: u64, original: &[u8], patched: &[u8]) -> Change {
        Change {
            anchor: Anchor::Offset(at),
            edit: Edit::Replace {
                original: Some(original.to_vec()),
                patched: patched.to_vec(),
            },
            label: String::new(),
        }
    }

    #[test]
    fn tables_keep_their_size_and_overlaps_are_refused() {
        let mut data = vec![1u8, 2, 3, 4];
        let grow = patch(vec![replace(0, &[1], &[9, 9])]);
        let report = apply(&mut data, &grow, None, true, true);
        assert_eq!((report.applied, data.len()), (0, 4), "{report:?}");
        let overlap = patch(vec![replace(0, &[1, 2], &[7, 7]), replace(1, &[2], &[8])]);
        let report = apply(&mut data, &overlap, None, false, true);
        assert_eq!(report.applied, 1);
        assert!(report.failed[0].contains("overlaps"), "{report:?}");
        assert_eq!(data, [7, 7, 3, 4]);
        let huge = patch(vec![replace(u64::MAX, &[1], &[2])]);
        assert_eq!(
            apply(&mut data, &huge, None, false, true).failed.len(),
            1,
            "no overflow panic"
        );
    }
}
