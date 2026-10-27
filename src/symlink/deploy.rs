//! Creating symlinks for a [`LinkPlan`].

use super::journal::Journal;
use super::{LinkRecord, Manifest, PathState, Report, bak_path, path_state, same_contents, stamp};
use crate::error::{IoContext, Result};
use crate::mod_engine::plan::{LinkOp, LinkPlan};
use crate::platform::LinkKind;
use std::fs;
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
    /// A stale Pailune link (pointing into staging) left by an interrupted
    /// run: remove it; an existing `.bak` is the original it shadowed.
    ReplaceOrphan(Option<PathBuf>),
    /// Cannot proceed without risking data loss.
    Refuse(String),
}

/// Decides how to free `target` for a new link.
fn displacement(target: &Path, owned: &[&Path]) -> Displace {
    let bak = bak_path(target);
    match (path_state(target), path_state(&bak)) {
        (PathState::Missing, _) => Displace::Nothing,
        (PathState::Symlink(dest), bak_state)
            if owned.iter().any(|root| dest.starts_with(root)) =>
        {
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
    owned: &[&Path],
    manifest: &mut Manifest,
    journal: &mut dyn Journal,
    link: Linker,
) -> Result<Option<String>> {
    let displace = displacement(&op.target, owned);
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
        stamp: None,
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
    let kind = link(&op.source, &op.target).at(&op.target)?;
    if kind != LinkKind::Symlink {
        // Not self-describing like a symlink: remember what we placed.
        let placed = stamp(&op.target);
        if let (Some(placed), Some(record)) = (placed, manifest.links.last_mut()) {
            journal.link_stamped(&op.target, &placed)?;
            record.stamp = Some(placed);
        }
    }
    Ok(None)
}

/// Places one file; the platform's [`crate::platform::link_file`] in production.
pub type Linker = fn(&Path, &Path) -> std::io::Result<LinkKind>;

/// Deploys every operation in `plan`, extending `manifest` as it goes.
///
/// Each link is journaled before the filesystem changes, so an interruption
/// at any point leaves a manifest that a later revert can undo exactly.
/// Stale links into an `owned` folder (staging, overlay) from interrupted
/// runs are replaced.
pub fn deploy(
    plan: &LinkPlan,
    owned: &[&Path],
    manifest: &mut Manifest,
    journal: &mut dyn Journal,
) -> Result<Report> {
    deploy_with(plan, owned, manifest, journal, crate::platform::link_file)
}

/// [`deploy`] with a chosen way of placing files (tests force copies).
pub fn deploy_with(
    plan: &LinkPlan,
    owned: &[&Path],
    manifest: &mut Manifest,
    journal: &mut dyn Journal,
    link: Linker,
) -> Result<Report> {
    let before = manifest.links.len();
    let warnings = plan
        .ops
        .iter()
        .map(|op| apply_op(op, owned, manifest, journal, link))
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
