//! Putting a save backup back in place.

use super::{SaveBackup, create, read_info};
use crate::error::{IoContext, PyluneError, Result};
use crate::mod_engine::install::unique_suffix;
use std::fs;
use std::path::Path;

/// Label of the safety backup taken before a restore.
pub const BEFORE_RESTORE: &str = "before restore";

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
    let has_saves = saves.is_dir() && fs::read_dir(saves).at(saves)?.next().is_some();
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
    fs::rename(&restored, saves).at(saves)?;
    let _ = fs::remove_dir_all(&work);
    let _ = fs::remove_dir_all(&old);
    Ok(safety)
}
