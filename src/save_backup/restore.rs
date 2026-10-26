//! Putting a save backup back in place.

use super::{SaveBackup, create, read_info, save_files};
use crate::error::{IoContext, PyluneError, Result};
use crate::mod_engine::install::unique_suffix;
use std::fs;
use std::path::Path;

/// Label of the safety backup taken before a restore.
pub const BEFORE_RESTORE: &str = "before restore";

/// Undoes an interrupted restore: when the save folder is missing but an
/// earlier restore had set it aside, it is moved back.
pub fn recover(saves: &Path) -> Result<()> {
    let Some(parent) = saves.parent() else {
        return Ok(());
    };
    let leftovers: Vec<_> = fs::read_dir(parent)
        .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    let named = |prefix: &str| {
        let mut found: Vec<&std::path::PathBuf> = leftovers
            .iter()
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(prefix))
            })
            .collect();
        found.sort();
        found
    };
    if !saves.exists()
        && let Some(old) = named(".pylune-old-saves-").last()
    {
        fs::rename(old, saves).at(saves)?;
    }
    named(".pylune-restore-")
        .iter()
        .for_each(|w| drop(fs::remove_dir_all(w)));
    Ok(())
}

/// Replaces the save folder with the backup's. The current saves are backed
/// up first (when there are any), and the swap is two renames, so the save
/// folder is never left half-written. Returns the safety backup.
pub fn restore(archive: &Path, saves: &Path, dest: &Path, now: i64) -> Result<Option<SaveBackup>> {
    read_info(archive).ok_or_else(|| {
        PyluneError::Invalid(format!("{} is not a save backup", archive.display()))
    })?;
    let parent = saves
        .parent()
        .ok_or_else(|| PyluneError::Invalid("the save folder has no parent".into()))?;
    recover(saves)?;
    let has_saves = !save_files(saves).is_empty();
    let safety = match has_saves {
        true => Some(create(saves, dest, BEFORE_RESTORE, now)?),
        false => None,
    };
    let suffix = unique_suffix();
    let work = parent.join(format!(".pylune-restore-{suffix}"));
    let extracted = (|| {
        zip::ZipArchive::new(fs::File::open(archive).at(archive)?)?.extract(&work)?;
        Ok::<_, PyluneError>(work.join("save"))
    })();
    let restored = match extracted {
        Ok(dir) if dir.is_dir() => dir,
        other => {
            let _ = fs::remove_dir_all(&work);
            other?;
            return Err(PyluneError::Invalid("the backup contains no saves".into()));
        }
    };
    let old = parent.join(format!(".pylune-old-saves-{suffix}"));
    if saves.exists() {
        fs::rename(saves, &old).at(saves)?;
    }
    if let Err(err) = fs::rename(&restored, saves) {
        // Put the original back rather than leave the game without saves.
        let _ = fs::rename(&old, saves);
        let _ = fs::remove_dir_all(&work);
        return Err(err).at(saves);
    }
    let _ = fs::remove_dir_all(&work);
    let _ = fs::remove_dir_all(&old);
    Ok(safety)
}
