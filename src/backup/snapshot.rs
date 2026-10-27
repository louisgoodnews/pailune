//! Creating and inspecting `.bak` snapshots.

use super::{BackupStatus, META_FILES, MetaBackup, VANILLA_LAST_ARCHIVE};
use crate::error::{IoContext, Result};
use crate::steam::SteamInfo;
use crate::symlink::{BAK_SUFFIX, Manifest, PathState, bak_path, path_state};
use std::fs;
use std::path::{Path, PathBuf};

/// Copies each real metadata file to `<file>.bak` unless a backup exists.
///
/// Returns how many backups were written.
pub fn ensure_meta_backup(info: &SteamInfo) -> Result<usize> {
    META_FILES
        .iter()
        .map(|name| info.meta_dir.join(name))
        .filter(|file| path_state(file) == PathState::File)
        .filter(|file| path_state(&bak_path(file)) == PathState::Missing)
        .map(|file| fs::copy(&file, bak_path(&file)).at(&file))
        .collect::<Result<Vec<_>>>()
        .map(|copied| copied.len())
}

/// Deletes the metadata snapshot and takes a fresh one from the current files.
pub fn refresh_meta_backup(info: &SteamInfo) -> Result<usize> {
    META_FILES
        .iter()
        .map(|name| bak_path(&info.meta_dir.join(name)))
        .filter(|bak| path_state(bak) == PathState::File)
        .try_for_each(|bak| fs::remove_file(&bak).at(&bak))?;
    ensure_meta_backup(info)
}

/// Compares the snapshotted game version with the installed one.
pub fn meta_backup_state(info: &SteamInfo) -> MetaBackup {
    let paver = info.meta_dir.join("0.paver");
    let papgt_bak = bak_path(&info.meta_dir.join("0.papgt"));
    match (
        papgt_bak.is_file(),
        fs::read(&paver).ok(),
        fs::read(bak_path(&paver)).ok(),
    ) {
        (false, _, _) => MetaBackup::Missing,
        (true, Some(now), Some(then)) if now != then => MetaBackup::Outdated,
        (true, _, _) => MetaBackup::Current,
    }
}

/// Parses a directory name like `0036` into its archive number.
fn archive_number(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    (name.len() == 4).then(|| name.parse().ok()).flatten()
}

/// Lists entries directly inside the game directory.
fn game_entries(info: &SteamInfo) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(&info.game_dir)
        .map(|dir| dir.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    entries.sort();
    entries
}

/// Real archive folders above [`VANILLA_LAST_ARCHIVE`] not created by Pailune.
pub fn foreign_overlays(info: &SteamInfo, manifest: &Manifest) -> Vec<PathBuf> {
    game_entries(info)
        .into_iter()
        .filter(|p| path_state(p) == PathState::Dir)
        .filter(|p| archive_number(p).is_some_and(|n| n > VANILLA_LAST_ARCHIVE))
        .filter(|p| !manifest.created_dirs.contains(p))
        .collect()
}

/// Archive folders previously set aside as `NNNN.bak`.
pub fn archived_overlays(info: &SteamInfo) -> Vec<PathBuf> {
    game_entries(info)
        .into_iter()
        .filter(|p| p.is_dir())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(BAK_SUFFIX))
                .is_some_and(|stem| archive_number(Path::new(stem)).is_some())
        })
        .collect()
}

/// Gathers everything the UI needs to show backup health.
pub fn status(info: &SteamInfo, manifest: &Manifest) -> BackupStatus {
    BackupStatus {
        meta: meta_backup_state(info),
        foreign_overlays: foreign_overlays(info, manifest),
        archived_overlays: archived_overlays(info),
    }
}
