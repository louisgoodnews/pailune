//! Creating symlinks for a [`LinkPlan`].

use super::journal::Journal;
use super::{LinkRecord, Manifest, PathState, Report, bak_path, path_state};
use crate::error::{IoContext, Result};
use crate::mod_engine::plan::{LinkOp, LinkPlan};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// How an occupied target is moved out of the way.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Displace {
    /// Target is free; just link.
    Nothing,
    /// Rename the original to its `.bak` path.
    RenameToBak(PathBuf),
    /// An identical `.bak` already exists; the original can be dropped.
    ReuseBak(PathBuf),
    /// A stale Pylune link (pointing into staging) left by an interrupted
    /// run: remove it; an existing `.bak` is the original it shadowed.
    ReplaceOrphan(Option<PathBuf>),
    /// Cannot proceed without risking data loss.
    Refuse(String),
}

/// Byte-for-byte comparison of two regular files.
fn same_contents(a: &Path, b: &Path) -> bool {
    let size = |p: &Path| fs::metadata(p).map(|m| m.len()).ok();
    size(a) == size(b) && size(a).is_some() && fs::read(a).ok() == fs::read(b).ok()
}

/// Decides how to free `target` for a new link.
fn displacement(target: &Path, staging: &Path) -> Displace {
    let bak = bak_path(target);
    match (path_state(target), path_state(&bak)) {
        (PathState::Missing, _) => Displace::Nothing,
        (PathState::Symlink(dest), bak_state) if dest.starts_with(staging) => {
            Displace::ReplaceOrphan((bak_state != PathState::Missing).then_some(bak))
        }
        (_, PathState::Missing) => Displace::RenameToBak(bak),
        (PathState::File, PathState::File) if same_contents(target, &bak) => {
            Displace::ReuseBak(bak)
        }
        _ => Displace::Refuse(format!(
            "{} is occupied and {} already exists; skipped",
            target.display(),
            bak.display()
        )),
    }
}

/// Missing parent directories of `target`, outermost first.
fn missing_parents(target: &Path) -> Vec<PathBuf> {
    let mut missing: Vec<PathBuf> = target
        .ancestors()
        .skip(1)
        .take_while(|dir| !dir.exists())
        .map(Path::to_path_buf)
        .collect();
    missing.reverse();
    missing
}

/// Performs a single link operation, journaling it before any change.
fn apply_op(
    op: &LinkOp,
    staging: &Path,
    manifest: &mut Manifest,
    journal: &mut dyn Journal,
) -> Result<Option<String>> {
    let displace = displacement(&op.target, staging);
    let backup = match &displace {
        Displace::Refuse(reason) => return Ok(Some(reason.clone())),
        Displace::Nothing => None,
        Displace::RenameToBak(bak) | Displace::ReuseBak(bak) => Some(bak.clone()),
        Displace::ReplaceOrphan(bak) => bak.clone(),
    };
    let record = LinkRecord {
        target: op.target.clone(),
        source: op.source.clone(),
        backup,
    };
    journal.link_planned(&record)?;
    manifest.links.push(record);
    match displace {
        Displace::RenameToBak(bak) => fs::rename(&op.target, &bak).at(&op.target)?,
        Displace::ReuseBak(_) | Displace::ReplaceOrphan(_) => {
            fs::remove_file(&op.target).at(&op.target)?
        }
        Displace::Nothing | Displace::Refuse(_) => {}
    }
    missing_parents(&op.target).iter().try_for_each(|dir| {
        journal.dir_planned(dir)?;
        manifest.created_dirs.push(dir.clone());
        fs::create_dir(dir).at(dir)
    })?;
    symlink(&op.source, &op.target).at(&op.target)?;
    Ok(None)
}

/// Deploys every operation in `plan`, extending `manifest` as it goes.
///
/// Each link is journaled before the filesystem changes, so an interruption
/// at any point leaves a manifest that a later revert can undo exactly.
/// Stale links into `staging` from interrupted runs are replaced.
pub fn deploy(
    plan: &LinkPlan,
    staging: &Path,
    manifest: &mut Manifest,
    journal: &mut dyn Journal,
) -> Result<Report> {
    let before = manifest.links.len();
    let warnings = plan
        .ops
        .iter()
        .map(|op| apply_op(op, staging, manifest, journal))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    let added = &manifest.links[before..];
    Ok(Report {
        linked: added.len(),
        backed_up: added.iter().filter(|r| r.backup.is_some()).count(),
        warnings,
    })
}
