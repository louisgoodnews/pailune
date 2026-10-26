//! Backups of the game's save files (board item A-28).
//!
//! Crimson Desert keeps its saves in `AppData/Local/Pearl Abyss/CD/save/<Steam id>/slot<N>/`
//! — on Linux inside the Proton prefix (`drive_c/users/steamuser/…`)
//! (`save.save`, `lobby.save`; a few MB in total, verified on build 2.3.2).
//! A backup is one `.zip` with a manifest and the whole `save/` tree.

mod restore;

pub use restore::{recover, restore};

use crate::app_backup::create::add_file;
use crate::error::{IoContext, PyluneError, Result};
use crate::mod_engine::install::unique_suffix;
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

/// Save folder relative to the Windows user's `AppData/Local`.
pub const SAVE_SUBDIR: &str = "Pearl Abyss/CD/save";

/// Version of the archive layout written into `manifest.json`.
const FORMAT: u64 = 1;

/// Label of backups taken automatically before Play.
pub const AUTO_LABEL: &str = "before play";

/// A save backup on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveBackup {
    pub path: PathBuf,
    pub created_at: i64,
    pub label: String,
    pub files: usize,
    pub size: u64,
}

/// The game's save folder: in `%LOCALAPPDATA%` on Windows, inside the
/// Proton/Wine prefix elsewhere (`None` while the prefix is unknown).
pub fn save_dir(prefix: Option<&Path>) -> Option<PathBuf> {
    crate::platform::local_app_data(prefix).map(|dir| dir.join(SAVE_SUBDIR))
}

/// Files below the save folder, relative to it.
pub(crate) fn save_files(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.path().strip_prefix(dir).ok().map(Path::to_path_buf))
        .collect()
}

/// Zips the save folder into `dest`.
pub fn create(saves: &Path, dest: &Path, label: &str, now: i64) -> Result<SaveBackup> {
    let files = save_files(saves);
    if files.is_empty() {
        return Err(PyluneError::Invalid(format!(
            "no save files found in {}",
            saves.display()
        )));
    }
    fs::create_dir_all(dest).at(dest)?;
    let target = dest.join(format!("saves-{now}-{}.zip", unique_suffix()));
    let part = target.with_extension("zip.part");
    let manifest = json!({
        "format": FORMAT, "kind": "saves", "created_at": now, "label": label,
        "files": files.len(), "app_version": env!("CARGO_PKG_VERSION"),
    });
    let written = (|| {
        let mut zip = zip::ZipWriter::new(fs::File::create(&part).at(&part)?);
        zip.start_file("manifest.json", SimpleFileOptions::default())?;
        zip.write_all(manifest.to_string().as_bytes()).at(&part)?;
        files.iter().try_for_each(|rel| {
            let name = format!("save/{}", rel.to_string_lossy().replace('\\', "/"));
            add_file(&mut zip, &name, &saves.join(rel))
        })?;
        zip.finish()?;
        fs::rename(&part, &target).at(&target)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&part);
    }
    written?;
    read_info(&target).ok_or_else(|| PyluneError::Invalid("save backup unreadable".into()))
}

/// Describes a save backup archive, if it is one.
pub fn read_info(path: &Path) -> Option<SaveBackup> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).ok()?).ok()?;
    let mut text = String::new();
    archive
        .by_name("manifest.json")
        .ok()?
        .read_to_string(&mut text)
        .ok()?;
    let m: Value = serde_json::from_str(&text).ok()?;
    (m.get("format")?.as_u64()? == FORMAT && m.get("kind")?.as_str()? == "saves").then_some(())?;
    Some(SaveBackup {
        path: path.to_path_buf(),
        created_at: m.get("created_at")?.as_i64()?,
        label: m.get("label")?.as_str()?.to_string(),
        files: m.get("files").and_then(Value::as_u64).unwrap_or(0) as usize,
        size: fs::metadata(path).map(|md| md.len()).unwrap_or(0),
    })
}

/// Save backups in `dir`, newest first.
pub fn list(dir: &Path) -> Vec<SaveBackup> {
    let mut found: Vec<SaveBackup> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "zip"))
                .filter_map(|p| read_info(&p))
                .collect()
        })
        .unwrap_or_default();
    found.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.path.cmp(&a.path)));
    found
}

/// Deletes all but the newest `keep` automatic save backups.
pub fn prune_auto(dir: &Path, keep: usize) -> Result<usize> {
    let old: Vec<PathBuf> = list(dir)
        .into_iter()
        .filter(|b| b.label == AUTO_LABEL)
        .skip(keep)
        .map(|b| b.path)
        .collect();
    old.iter().try_for_each(|p| fs::remove_file(p).at(p))?;
    Ok(old.len())
}

#[cfg(test)]
mod tests;
