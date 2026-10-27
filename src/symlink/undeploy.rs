//! Removing deployed symlinks and restoring their `.bak` originals.

use super::journal::Journal;
use super::{LinkRecord, Manifest, PathState, Report, path_state, same_contents, stamp};
use crate::error::{IoContext, Result};
use std::fs;

/// Outcome of reverting a single [`LinkRecord`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum Reverted {
    /// Link removed, nothing to restore.
    Unlinked,
    /// Link removed (or already gone) and original restored from `.bak`.
    Restored,
    /// The record was journaled but the change never happened (interrupted run).
    NotApplied,
    /// Left untouched; explains why.
    Skipped(String),
}

/// What currently sits at the record's target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// Our symlink.
    Ours,
    /// Nothing.
    Free,
    /// Something else (the untouched original, or a replacement).
    Foreign,
}

/// What occupies the record's target now.
///
/// A regular file is ours when it still carries the stamp recorded after it
/// was placed (hard links, copies). Without a stamp (a run interrupted right
/// after placing it) identical content counts — unless the planned `.bak`
/// rename never happened, because then the file is the untouched original.
fn target_of(record: &LinkRecord) -> Target {
    let bak_pending = record
        .backup
        .as_ref()
        .is_some_and(|bak| path_state(bak) == PathState::Missing);
    match (path_state(&record.target), record.stamp) {
        (PathState::Symlink(dest), _) if dest == record.source => Target::Ours,
        (PathState::Missing, _) => Target::Free,
        (PathState::File, Some(placed)) if stamp(&record.target) == Some(placed) => Target::Ours,
        (PathState::File, None)
            if !bak_pending && same_contents(&record.target, &record.source) =>
        {
            Target::Ours
        }
        _ => Target::Foreign,
    }
}

/// Reverts one record. Only removes the target if it is still *our* link.
fn revert(record: &LinkRecord) -> Result<Reverted> {
    let target = target_of(record);
    let bak = record
        .backup
        .as_ref()
        .filter(|bak| path_state(bak) != PathState::Missing);
    if target == Target::Ours {
        fs::remove_file(&record.target).at(&record.target)?;
    }
    match (target, bak, &record.backup) {
        (Target::Ours | Target::Free, Some(bak), _) => {
            fs::rename(bak, &record.target).at(bak)?;
            Ok(Reverted::Restored)
        }
        (Target::Ours | Target::Free, None, None) => Ok(Reverted::Unlinked),
        (Target::Ours | Target::Free, None, Some(missing)) => Ok(Reverted::Skipped(format!(
            "backup {} is missing",
            missing.display()
        ))),
        // The original is still in place: the run stopped before touching it.
        (Target::Foreign, None, _) => Ok(Reverted::NotApplied),
        (Target::Foreign, Some(_), _) => Ok(Reverted::Skipped(format!(
            "{} was replaced by something else; left as is (its .bak was kept)",
            record.target.display()
        ))),
    }
}

/// Reverts every record in `manifest` (newest first) and empties it.
///
/// Each record leaves the journal only after it was reverted, so an
/// interruption keeps the remaining records for the next attempt.
/// Directories Pailune created are removed again if they ended up empty.
pub fn undeploy(manifest: &mut Manifest, journal: &mut dyn Journal) -> Result<Report> {
    let outcomes = std::iter::from_fn(|| manifest.links.pop())
        .map(|record| {
            let outcome = revert(&record)?;
            journal.link_reverted(&record.target)?;
            Ok(outcome)
        })
        .collect::<Result<Vec<_>>>()?;
    manifest
        .created_dirs
        .iter()
        .rev()
        .for_each(|dir| drop(fs::remove_dir(dir)));
    manifest.created_dirs.clear();
    journal.dirs_cleared()?;
    Ok(outcomes
        .into_iter()
        .fold(Report::default(), |mut report, o| {
            match o {
                Reverted::Unlinked => report.linked += 1,
                Reverted::Restored => {
                    report.linked += 1;
                    report.backed_up += 1;
                }
                Reverted::NotApplied => {}
                Reverted::Skipped(reason) => report.warnings.push(reason),
            }
            report
        }))
}
