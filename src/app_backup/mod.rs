//! Backups of Pylune itself: settings, the database (mod list, load order,
//! flags, variants, profiles, collections, journal) and optionally the
//! staged mod files, as one `.zip` per backup.
//!
//! Not to be confused with [`crate::backup`], which protects the *game's*
//! metadata. The Nexus API key is never included.

pub mod create;
pub mod restore;

use std::path::PathBuf;

/// Version of the backup layout written into `manifest.json`.
pub const FORMAT: u64 = 1;

/// Label of automatic backups.
pub const AUTO_LABEL: &str = "auto";

/// What a backup contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupKind {
    /// Settings and database: small, quick.
    Settings,
    /// Settings, database and every staged mod file.
    Full,
}

/// A backup on disk, as described by its manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub created_at: i64,
    pub kind: BackupKind,
    /// `auto`, `before restore`, or the user's label.
    pub label: String,
    /// Mods in the backed-up mod list.
    pub mods: usize,
    pub size: u64,
    pub app_version: String,
}

/// What a restore changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreReport {
    pub settings: bool,
    pub mod_files: bool,
    /// Where the replaced staging folder was moved (full restores).
    pub previous_staging: Option<PathBuf>,
}

impl BackupKind {
    /// Name used in manifests and file names.
    pub fn as_str(self) -> &'static str {
        match self {
            BackupKind::Settings => "settings",
            BackupKind::Full => "full",
        }
    }

    /// Kind from its manifest name.
    pub fn parse(text: &str) -> Self {
        match text {
            "full" => BackupKind::Full,
            _ => BackupKind::Settings,
        }
    }
}
