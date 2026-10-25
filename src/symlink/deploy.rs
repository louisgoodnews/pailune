//! Creating symlinks for a [`LinkPlan`].

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
    /// Cannot proceed without risking data loss.
    Refuse(String),
}

/// Byte-for-byte comparison of two regular files.
fn same_contents(a: &Path, b: &Path) -> bool {
    let size = |p: &Path| fs::metadata(p).map(|m| m.len()).ok();
    size(a) == size(b) && size(a).is_some() && fs::read(a).ok() == fs::read(b).ok()
}

/// Decides how to free `target` for a new link.
fn displacement(target: &Path) -> Displace {
    let bak = bak_path(target);
    match (path_state(target), path_state(&bak)) {
        (PathState::Missing, _) => Displace::Nothing,
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

/// Creates missing parent directories of `target`, returning those created.
fn ensure_parents(target: &Path) -> Result<Vec<PathBuf>> {
    let missing: Vec<PathBuf> = target
        .ancestors()
        .skip(1)
        .take_while(|dir| !dir.exists())
        .map(Path::to_path_buf)
        .collect();
    missing
        .iter()
        .rev()
        .try_for_each(|dir| fs::create_dir(dir).at(dir))?;
    Ok(missing.into_iter().rev().collect())
}

/// Performs a single link operation, appending to `manifest` on success.
fn apply_op(op: &LinkOp, manifest: &mut Manifest) -> Result<Option<String>> {
    let backup = match displacement(&op.target) {
        Displace::Refuse(reason) => return Ok(Some(reason)),
        Displace::Nothing => None,
        Displace::RenameToBak(bak) => {
            fs::rename(&op.target, &bak).at(&op.target)?;
            Some(bak)
        }
        Displace::ReuseBak(bak) => {
            fs::remove_file(&op.target).at(&op.target)?;
            Some(bak)
        }
    };
    manifest.created_dirs.extend(ensure_parents(&op.target)?);
    symlink(&op.source, &op.target).at(&op.target)?;
    manifest.links.push(LinkRecord {
        target: op.target.clone(),
        source: op.source.clone(),
        backup,
    });
    Ok(None)
}

/// Deploys every operation in `plan`, extending `manifest` as it goes.
///
/// The manifest is updated even when an operation fails midway, so the
/// caller can persist it and a later revert stays exact.
pub fn deploy(plan: &LinkPlan, manifest: &mut Manifest) -> Result<Report> {
    let before = manifest.links.len();
    let warnings = plan
        .ops
        .iter()
        .map(|op| apply_op(op, manifest))
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
