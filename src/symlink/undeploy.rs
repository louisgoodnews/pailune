//! Removing deployed symlinks and restoring their `.bak` originals.

use super::{LinkRecord, Manifest, PathState, Report, path_state};
use crate::error::{IoContext, Result};
use std::fs;

/// Outcome of reverting a single [`LinkRecord`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum Reverted {
    /// Link removed, nothing to restore.
    Unlinked,
    /// Link removed and original restored from `.bak`.
    Restored,
    /// Left untouched; explains why.
    Skipped(String),
}

/// Reverts one record. Only removes the target if it is still *our* link.
fn revert(record: &LinkRecord) -> Result<Reverted> {
    let ours = matches!(path_state(&record.target),
        PathState::Symlink(dest) if dest == record.source);
    let target_free = ours || path_state(&record.target) == PathState::Missing;
    if ours {
        fs::remove_file(&record.target).at(&record.target)?;
    }
    match (&record.backup, target_free) {
        (_, false) => Ok(Reverted::Skipped(format!(
            "{} was replaced by something else; left as is",
            record.target.display()
        ))),
        (Some(bak), true) if path_state(bak) != PathState::Missing => {
            fs::rename(bak, &record.target).at(bak)?;
            Ok(Reverted::Restored)
        }
        (Some(bak), true) => Ok(Reverted::Skipped(format!(
            "backup {} is missing",
            bak.display()
        ))),
        (None, true) => Ok(Reverted::Unlinked),
    }
}

/// Reverts every record in `manifest` (newest first) and empties it.
///
/// Directories Pylune created are removed again if they ended up empty.
pub fn undeploy(manifest: &mut Manifest) -> Result<Report> {
    let outcomes = manifest
        .links
        .iter()
        .rev()
        .map(revert)
        .collect::<Result<Vec<_>>>()?;
    manifest
        .created_dirs
        .iter()
        .rev()
        .for_each(|dir| drop(fs::remove_dir(dir)));
    *manifest = Manifest::default();
    Ok(outcomes
        .into_iter()
        .fold(Report::default(), |mut report, o| {
            match o {
                Reverted::Unlinked => report.linked += 1,
                Reverted::Restored => {
                    report.linked += 1;
                    report.backed_up += 1;
                }
                Reverted::Skipped(reason) => report.warnings.push(reason),
            }
            report
        }))
}
