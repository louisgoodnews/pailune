//! Locating Pylune's data directory and (de)serialising JSON state files.

use crate::error::{IoContext, PyluneError, Result};
use directories::ProjectDirs;
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::{Path, PathBuf};

/// Well-known locations inside Pylune's data directory (`~/.local/share/pylune`).
#[derive(Debug, Clone)]
pub struct DataPaths {
    /// Staging folder holding one sub-directory per installed mod.
    pub staging: PathBuf,
    /// User settings.
    pub config: PathBuf,
    /// Mod list with enabled flags and load order.
    pub registry: PathBuf,
    /// Record of every symlink currently deployed into the game.
    pub manifest: PathBuf,
}

/// Resolves (and creates) the data directory layout.
pub fn data_paths() -> Result<DataPaths> {
    let root = ProjectDirs::from("dev", "pylune", "pylune")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or_else(|| PyluneError::Invalid("cannot determine home directory".into()))?;
    let paths = DataPaths {
        staging: root.join("staging"),
        config: root.join("config.json"),
        registry: root.join("registry.json"),
        manifest: root.join("manifest.json"),
    };
    fs::create_dir_all(&paths.staging).at(&paths.staging)?;
    Ok(paths)
}

/// Reads a JSON file, falling back to `T::default()` when it does not exist.
pub fn load_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(err) => Err(err).at(path),
    }
}

/// Writes `value` as pretty JSON, atomically via a temporary sibling file.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(value)?).at(&tmp)?;
    fs::rename(&tmp, path).at(path)
}
