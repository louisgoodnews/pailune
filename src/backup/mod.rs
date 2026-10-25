//! Game-level safety net: `.bak` snapshots of archive metadata and a
//! "Clean Slate" revert that returns the installation to vanilla.
//!
//! *Crimson Desert* indexes its archive folders (`0000`…`0035`) through
//! `meta/0.papgt`. Mods typically add folders numbered above 35 and patch the
//! metadata, so those are exactly the targets protected here.

pub mod reset;
pub mod snapshot;

use std::path::PathBuf;

/// Metadata files snapshotted to `.bak`. `0.paver` is the game version and is
/// used to detect snapshots made before a game update.
pub const META_FILES: [&str; 3] = ["0.papgt", "0.pathc", "0.paver"];

/// Highest archive folder number shipped with the game.
pub const VANILLA_LAST_ARCHIVE: u32 = 35;

/// State of the metadata snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaBackup {
    /// No snapshot exists yet.
    Missing,
    /// Snapshot matches the installed game version.
    Current,
    /// The game was updated after the snapshot; restoring it would break.
    Outdated,
}

/// Overview of backup-relevant state, shown in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupStatus {
    /// Metadata snapshot health.
    pub meta: MetaBackup,
    /// Non-vanilla archive folders (> 35) not created by Pylune.
    pub foreign_overlays: Vec<PathBuf>,
    /// Archive folders previously set aside as `NNNN.bak`.
    pub archived_overlays: Vec<PathBuf>,
}
