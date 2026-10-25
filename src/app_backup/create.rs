//! Writing, listing and pruning backups.

use super::{AUTO_LABEL, BackupInfo, BackupKind, FORMAT};
use crate::config::paths::DataPaths;
use crate::error::{IoContext, PyluneError, Result};
use crate::mod_engine::install::unique_suffix;
use rusqlite::{Connection, MAIN_DB};
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

/// Adds one file to the archive.
pub(crate) fn add_file(zip: &mut zip::ZipWriter<fs::File>, name: &str, path: &Path) -> Result<()> {
    zip.start_file(name, SimpleFileOptions::default())?;
    let mut source = fs::File::open(path).at(path)?;
    std::io::copy(&mut source, zip).at(path).map(drop)
}

/// Adds every staged mod file (hidden work folders are skipped).
fn add_staging(zip: &mut zip::ZipWriter<fs::File>, staging: &Path) -> Result<()> {
    WalkDir::new(staging)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with(".pylune"))
        .filter(|e| e.as_ref().map_or(true, |e| e.file_type().is_file()))
        .try_for_each(|entry| {
            let entry = entry?;
            let rel = entry.path().strip_prefix(staging).unwrap_or(entry.path());
            add_file(
                zip,
                &format!("staging/{}", rel.to_string_lossy().replace('\\', "/")),
                entry.path(),
            )
        })
}

/// Writes a backup into `paths.backups`; the database is copied with
/// SQLite's online backup, so a running Pylune is captured consistently.
pub fn create(
    paths: &DataPaths,
    conn: &Connection,
    kind: BackupKind,
    label: &str,
    now: i64,
) -> Result<BackupInfo> {
    fs::create_dir_all(&paths.backups).at(&paths.backups)?;
    let snapshot = paths.backups.join(format!(".snapshot-{}", unique_suffix()));
    conn.backup(MAIN_DB, &snapshot, None::<fn(rusqlite::backup::Progress)>)?;
    let mods = crate::db::mods::load(conn)?.mods.len();
    let target = paths.backups.join(format!(
        "pylune-{now}-{}-{}.zip",
        kind.as_str(),
        unique_suffix()
    ));
    let part = target.with_extension("zip.part");
    let manifest = json!({
        "format": FORMAT, "created_at": now, "kind": kind.as_str(), "label": label,
        "mods": mods, "app_version": env!("CARGO_PKG_VERSION"),
    });
    let written = (|| {
        let mut zip = zip::ZipWriter::new(fs::File::create(&part).at(&part)?);
        zip.start_file("manifest.json", SimpleFileOptions::default())?;
        zip.write_all(manifest.to_string().as_bytes()).at(&part)?;
        add_file(&mut zip, "pylune.sqlite3", &snapshot)?;
        if paths.config_file.is_file() {
            add_file(&mut zip, "config.toml", &paths.config_file)?;
        }
        if kind == BackupKind::Full {
            add_staging(&mut zip, &paths.staging)?;
        }
        zip.finish()?;
        fs::rename(&part, &target).at(&target)
    })();
    let _ = fs::remove_file(&snapshot);
    if written.is_err() {
        let _ = fs::remove_file(&part);
    }
    written?;
    read_info(&target).ok_or_else(|| PyluneError::Invalid("backup written but unreadable".into()))
}

/// Reads the manifest of a backup archive.
pub fn read_manifest(path: &Path) -> Result<Value> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).at(path)?)?;
    let mut text = String::new();
    archive
        .by_name("manifest.json")?
        .read_to_string(&mut text)
        .at(path)?;
    let manifest: Value = serde_json::from_str(&text)?;
    match manifest.get("format").and_then(Value::as_u64) {
        Some(FORMAT) => Ok(manifest),
        _ => Err(PyluneError::Invalid(format!(
            "{} is not a Pylune backup (or a newer format)",
            path.display()
        ))),
    }
}

/// Describes a backup archive, if it is one.
pub fn read_info(path: &Path) -> Option<BackupInfo> {
    let m = read_manifest(path).ok()?;
    Some(BackupInfo {
        path: path.to_path_buf(),
        created_at: m.get("created_at")?.as_i64()?,
        kind: BackupKind::parse(m.get("kind")?.as_str()?),
        label: m
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        mods: m.get("mods").and_then(Value::as_u64).unwrap_or(0) as usize,
        size: fs::metadata(path).map(|md| md.len()).unwrap_or(0),
        app_version: m
            .get("app_version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

/// Backups in `dir`, newest first.
pub fn list(dir: &Path) -> Vec<BackupInfo> {
    let mut found: Vec<BackupInfo> = fs::read_dir(dir)
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

/// Deletes all but the newest `keep` automatic backups.
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

/// Whether an automatic backup is due (none in the last 24 hours).
pub fn auto_due(dir: &Path, now: i64) -> bool {
    list(dir)
        .iter()
        .filter(|b| b.label == AUTO_LABEL)
        .all(|b| now - b.created_at > 24 * 3600)
}
