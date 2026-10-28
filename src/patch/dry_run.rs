//! Dry-run verification of JSON mods against installed game files.
//!
//! Checks format-2 byte patches and format-3 field intents against the actual
//! game files before Apply:
//! - Records and fields exist in the target table
//! - Original bytes match the bytes installed on disk
//! - Table modifications preserve their byte length
//! - Changes do not overlap each other within the same file

use super::apply::Frame;
use super::{Anchor, Edit, FilePatch, JsonMod};
use crate::tables::format3;
use std::path::Path;

/// Status of an individual change or intent during dry run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DryRunStatus {
    /// Change verified against the game file cleanly.
    Verified,
    /// Change failed verification (with reason).
    Failed(String),
    /// Game file could not be found or verified.
    Skipped(String),
}

/// Suggestion to update a format-2 change's relative offset when original bytes moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseSuggestion {
    pub entry_name: String,
    pub old_offset: u64,
    pub new_offset: u64,
}

/// Verification results for a single game file patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchDryRun {
    pub file_name: String,
    pub file_found: bool,
    pub changes: Vec<DryRunStatus>,
    pub intents: Vec<DryRunStatus>,
    pub rebases: std::collections::HashMap<usize, RebaseSuggestion>,
}

/// Verification results for an entire JSON mod.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModDryRun {
    pub patches: Vec<PatchDryRun>,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

/// Verifies a patch in-memory against provided table/file bytes.
pub fn check_patch_data(
    patch: &FilePatch,
    header: Option<&[u8]>,
    data: &[u8],
    is_table: bool,
) -> PatchDryRun {
    let mut occupied_spans: Vec<(usize, usize)> = Vec::new();
    let mut intents = Vec::with_capacity(patch.fields.len());

    // 1. Check format-3 field intents
    for intent in &patch.fields {
        let Some(header_bytes) = header else {
            intents.push(DryRunStatus::Failed(
                "Format-3 intent requires a staticinfo data table".into(),
            ));
            continue;
        };

        let (changes, errs) = format3::resolve(&patch.game_file, header_bytes, data, std::slice::from_ref(intent));
        if !errs.is_empty() || changes.is_empty() {
            let reason = match errs.is_empty() {
                true => "Record or field could not be resolved".to_string(),
                false => errs.join("; "),
            };
            intents.push(DryRunStatus::Failed(reason));
            continue;
        }

        // Test resolved change
        let mut intent_ok = true;
        for c in &changes {
            let at = match &c.anchor {
                Anchor::Offset(o) => *o as usize,
                Anchor::Entry { .. } => 0,
            };
            let span_len = match &c.edit {
                Edit::Replace { original, patched } => {
                    original.as_ref().map_or(patched.len(), Vec::len)
                }
                Edit::Insert { bytes } => bytes.len(),
            };
            let end = at.saturating_add(span_len);
            if end > data.len() {
                intents.push(DryRunStatus::Failed("Resolved offset beyond file bounds".into()));
                intent_ok = false;
                break;
            }
            if occupied_spans.iter().any(|&(s, e)| at < e && s < end) {
                intents.push(DryRunStatus::Failed("Overlaps an earlier change".into()));
                intent_ok = false;
                break;
            }
            occupied_spans.push((at, end));
        }

        if intent_ok {
            intents.push(DryRunStatus::Verified);
        }
    }

    // 2. Check format-2 byte patches
    let mut changes = Vec::with_capacity(patch.changes.len());
    let mut rebases = std::collections::HashMap::new();
    let frame = Frame::of(data, patch, header);

    for (c, change) in patch.changes.iter().enumerate() {
        let Some(ref f) = frame else {
            changes.push(DryRunStatus::Failed("Anchor signature not found in file".into()));
            continue;
        };

        let Some(at) = f.position(change) else {
            changes.push(DryRunStatus::Failed("Entry not found in table".into()));
            continue;
        };

        match &change.edit {
            Edit::Insert { .. } if is_table => {
                changes.push(DryRunStatus::Failed(
                    "Inserts into data tables are not supported".into(),
                ));
            }
            Edit::Insert { bytes } => {
                if at > data.len() {
                    changes.push(DryRunStatus::Failed("Offset beyond end of file".into()));
                } else {
                    let end = at + bytes.len();
                    if occupied_spans.iter().any(|&(s, e)| at < e && s < end) {
                        changes.push(DryRunStatus::Failed("Overlaps an earlier change".into()));
                    } else {
                        occupied_spans.push((at, end));
                        changes.push(DryRunStatus::Verified);
                    }
                }
            }
            Edit::Replace { original, patched } => {
                if is_table && original.as_ref().is_some_and(|o| o.len() != patched.len()) && header.is_none() {
                    changes.push(DryRunStatus::Failed(
                        "Changes to data tables without an index must keep their length".into(),
                    ));
                    continue;
                }

                let span_len = original.as_ref().map_or(patched.len(), Vec::len);
                let end = match at.checked_add(span_len) {
                    Some(e) if e <= data.len() => e,
                    _ => {
                        changes.push(DryRunStatus::Failed("Offset beyond end of file".into()));
                        continue;
                    }
                };

                if let Some(orig) = original {
                    if data.get(at..end) != Some(orig.as_slice()) {
                        // Check if bytes moved within this record
                        let mut found_rebase = None;
                        if let Anchor::Entry { ref name, rel_offset } = change.anchor {
                            if let Some(ref anchors) = f.anchors {
                                if let Some(&name_at) = anchors.get(name) {
                                    let search_end = (name_at + 16384).min(data.len());
                                    if search_end > name_at {
                                        let slice = &data[name_at..search_end];
                                        if let Some(pos) = slice
                                            .windows(orig.len())
                                            .position(|w| w == orig.as_slice())
                                        {
                                            if (pos as u64) != rel_offset {
                                                found_rebase = Some(RebaseSuggestion {
                                                    entry_name: name.clone(),
                                                    old_offset: rel_offset,
                                                    new_offset: pos as u64,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(sug) = found_rebase {
                            rebases.insert(c, sug.clone());
                            changes.push(DryRunStatus::Failed(format!(
                                "Original bytes moved to +{} (was +{})",
                                sug.new_offset, sug.old_offset
                            )));
                        } else {
                            changes.push(DryRunStatus::Failed(
                                "Original bytes do not match installed game data".into(),
                            ));
                        }
                        continue;
                    }
                }

                if occupied_spans.iter().any(|&(s, e)| at < e && s < end) {
                    changes.push(DryRunStatus::Failed("Overlaps an earlier change".into()));
                } else {
                    occupied_spans.push((at, end));
                    changes.push(DryRunStatus::Verified);
                }
            }
        }
    }

    PatchDryRun {
        file_name: patch.game_file.clone(),
        file_found: true,
        changes,
        intents,
        rebases,
    }
}

/// Verifies a JSON mod against the game installation on disk.
pub fn check_mod(game_dir: &Path, json_mod: &JsonMod) -> ModDryRun {
    let mut patches = Vec::with_capacity(json_mod.patches.len());
    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    for patch in &json_mod.patches {
        total += patch.changes.len() + patch.fields.len();

        let norm_file = patch.game_file.replace('\\', "/");
        let stem = norm_file
            .rsplit('/')
            .next()
            .unwrap_or(&norm_file)
            .split('.')
            .next()
            .unwrap_or("");

        // Check if table in archive group 0008
        let patch_res = if let Ok((header, body)) = crate::tables::browser::read_table_raw(game_dir, stem) {
            check_patch_data(patch, Some(&header), &body, true)
        } else if let Ok(data) = std::fs::read(game_dir.join(&patch.game_file)) {
            check_patch_data(patch, None, &data, false)
        } else {
            PatchDryRun {
                file_name: patch.game_file.clone(),
                file_found: false,
                changes: vec![
                    DryRunStatus::Skipped("Game file not found in archives or game folder".into());
                    patch.changes.len()
                ],
                intents: vec![
                    DryRunStatus::Skipped("Game file not found in archives or game folder".into());
                    patch.fields.len()
                ],
                rebases: std::collections::HashMap::new(),
            }
        };

        for status in &patch_res.changes {
            match status {
                DryRunStatus::Verified => passed += 1,
                DryRunStatus::Failed(_) => failed += 1,
                DryRunStatus::Skipped(_) => {}
            }
        }
        for status in &patch_res.intents {
            match status {
                DryRunStatus::Verified => passed += 1,
                DryRunStatus::Failed(_) => failed += 1,
                DryRunStatus::Skipped(_) => {}
            }
        }

        patches.push(patch_res);
    }

    ModDryRun {
        patches,
        total,
        passed,
        failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{Anchor, Change, Edit};

    #[test]
    fn test_format2_dry_run_success_and_mismatch() {
        let original_data = vec![0x10, 0x20, 0x30, 0x40, 0x50, 0x60];
        let patch = FilePatch {
            game_file: "test.bin".into(),
            signature: None,
            changes: vec![
                Change {
                    anchor: Anchor::Offset(1),
                    edit: Edit::Replace {
                        original: Some(vec![0x20, 0x30]),
                        patched: vec![0xaa, 0xbb],
                    },
                    label: "valid replace".into(),
                },
                Change {
                    anchor: Anchor::Offset(4),
                    edit: Edit::Replace {
                        original: Some(vec![0xff]), // mismatch!
                        patched: vec![0xee],
                    },
                    label: "mismatched replace".into(),
                },
            ],
            fields: Vec::new(),
        };

        let res = check_patch_data(&patch, None, &original_data, false);
        assert_eq!(res.changes.len(), 2);
        assert_eq!(res.changes[0], DryRunStatus::Verified);
        assert!(matches!(res.changes[1], DryRunStatus::Failed(_)));
    }

    #[test]
    fn test_format2_dry_run_detects_overlap() {
        let original_data = vec![0x10, 0x20, 0x30, 0x40];
        let patch = FilePatch {
            game_file: "test.bin".into(),
            signature: None,
            changes: vec![
                Change {
                    anchor: Anchor::Offset(0),
                    edit: Edit::Replace {
                        original: Some(vec![0x10, 0x20]),
                        patched: vec![0xaa, 0xbb],
                    },
                    label: "first".into(),
                },
                Change {
                    anchor: Anchor::Offset(1),
                    edit: Edit::Replace {
                        original: Some(vec![0x20, 0x30]),
                        patched: vec![0xcc, 0xdd],
                    },
                    label: "overlapping second".into(),
                },
            ],
            fields: Vec::new(),
        };

        let res = check_patch_data(&patch, None, &original_data, false);
        assert_eq!(res.changes.len(), 2);
        assert_eq!(res.changes[0], DryRunStatus::Verified);
        assert_eq!(
            res.changes[1],
            DryRunStatus::Failed("Overlaps an earlier change".into())
        );
    }
}
