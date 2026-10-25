//! Well-known file locations (XDG data and config directories).

use crate::error::{IoContext, PyluneError, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

/// Every path Pylune reads or writes outside the game directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPaths {
    /// `~/.config/pylune/config.toml`
    pub config_file: PathBuf,
    /// SQLite database holding all application state.
    pub database: PathBuf,
    /// One sub-directory per installed mod.
    pub staging: PathBuf,
    /// Temporary location for downloads.
    pub downloads: PathBuf,
    /// Pylune's generated archive group and `0.papgt`, linked into the game.
    pub overlay: PathBuf,
    /// Deleted mods, restorable until pruned.
    pub trash: PathBuf,
    /// Backups of Pylune's settings, database and mod files.
    pub backups: PathBuf,
    /// Backups of the game's save files.
    pub save_backups: PathBuf,
    /// Rolling application log files.
    pub logs: PathBuf,
    /// One log file per mod.
    pub mod_logs: PathBuf,
    /// Nexus API key (mode 0600).
    pub nexus_key: PathBuf,
    /// Requests (`nxm://` links, focus) handed over by secondary instances.
    pub spool: PathBuf,
    /// Held with `flock` by the running primary instance.
    pub lock: PathBuf,
}

impl DataPaths {
    /// Lays out all paths below the given data and config roots.
    pub fn under(data: &Path, config: &Path) -> Self {
        Self {
            config_file: config.join("config.toml"),
            database: data.join("pylune.sqlite3"),
            staging: data.join("staging"),
            downloads: data.join("downloads"),
            overlay: data.join("overlay"),
            trash: data.join("trash"),
            backups: data.join("backups"),
            save_backups: data.join("save-backups"),
            logs: data.join("logs"),
            mod_logs: data.join("logs").join("mods"),
            nexus_key: data.join("nexus_api_key"),
            spool: data.join("spool"),
            lock: data.join("pylune.lock"),
        }
    }

    /// Creates every directory in the layout.
    pub fn create_dirs(&self) -> Result<()> {
        [&self.staging, &self.downloads, &self.mod_logs]
            .into_iter()
            .try_for_each(|dir| fs::create_dir_all(dir).at(dir))
    }
}

/// Resolves (and creates) the XDG layout: `~/.local/share/pylune`, `~/.config/pylune`.
pub fn data_paths() -> Result<DataPaths> {
    let dirs = ProjectDirs::from("dev", "pylune", "pylune")
        .ok_or_else(|| PyluneError::Invalid("cannot determine home directory".into()))?;
    let paths = DataPaths::under(dirs.data_dir(), dirs.config_dir());
    paths.create_dirs()?;
    Ok(paths)
}
