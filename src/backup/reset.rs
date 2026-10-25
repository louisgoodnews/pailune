//! The "Clean Slate" revert and its inverse.

use super::snapshot::{archived_overlays, ensure_meta_backup, foreign_overlays, meta_backup_state};
use super::{META_FILES, MetaBackup};
use crate::error::{IoContext, PyluneError, Result};
use crate::steam::SteamInfo;
use crate::symlink::journal::Journal;
use crate::symlink::undeploy::undeploy;
use crate::symlink::{BAK_SUFFIX, Manifest, PathState, Report, bak_path, path_state};
use std::fs;
use std::path::Path;

/// Restores every metadata file from its `.bak` copy (the copy is kept).
fn restore_meta(info: &SteamInfo) -> Result<usize> {
    match meta_backup_state(info) {
        MetaBackup::Outdated => Err(PyluneError::Invalid(
            "metadata backup predates the installed game version; \
             verify game files in Steam instead, then refresh the backup"
                .into(),
        )),
        MetaBackup::Missing => Ok(0),
        MetaBackup::Current => META_FILES
            .iter()
            .map(|name| info.meta_dir.join(name))
            .filter(|file| path_state(&bak_path(file)) == PathState::File)
            // Never write through a symlink into a staged mod file.
            .filter(|file| matches!(path_state(file), PathState::File | PathState::Missing))
            .map(|file| fs::copy(bak_path(&file), &file).at(&file))
            .collect::<Result<Vec<_>>>()
            .map(|restored| restored.len()),
    }
}

/// Renames a directory to `<dir>.bak`, refusing to overwrite.
fn archive_dir(dir: &Path) -> Result<Option<String>> {
    let bak = bak_path(dir);
    match path_state(&bak) {
        PathState::Missing => fs::rename(dir, &bak).at(dir).map(|()| None),
        _ => Ok(Some(format!(
            "{} already exists; left {} in place",
            bak.display(),
            dir.display()
        ))),
    }
}

/// Returns the game to a vanilla state.
///
/// 1. Removes all Pylune symlinks and restores shadowed originals.
/// 2. Restores `meta/` files from their `.bak` snapshot.
/// 3. Sets foreign archive folders (> 35) aside as `NNNN.bak`.
/// 4. Re-takes the metadata snapshot if step 1 consumed it.
pub fn clean_slate(
    info: &SteamInfo,
    manifest: &mut Manifest,
    journal: &mut dyn Journal,
) -> Result<Report> {
    let mut report = undeploy(manifest, journal)?;
    report.backed_up += restore_meta(info)?;
    let overlay_warnings = foreign_overlays(info, manifest)
        .iter()
        .map(|dir| archive_dir(dir))
        .collect::<Result<Vec<_>>>()?;
    report
        .warnings
        .extend(overlay_warnings.into_iter().flatten());
    ensure_meta_backup(info)?;
    Ok(report)
}

/// Moves `NNNN.bak` folders back to `NNNN` where that name is free.
pub fn restore_overlays(info: &SteamInfo) -> Result<usize> {
    archived_overlays(info)
        .iter()
        .filter_map(|bak| {
            let name = bak.file_name()?.to_str()?.strip_suffix(BAK_SUFFIX)?;
            Some((bak.clone(), bak.with_file_name(name)))
        })
        .filter(|(_, dir)| path_state(dir) == PathState::Missing)
        .map(|(bak, dir)| fs::rename(&bak, &dir).at(&bak))
        .collect::<Result<Vec<_>>>()
        .map(|restored| restored.len())
}
