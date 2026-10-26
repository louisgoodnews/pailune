//! Steam / Proton path resolution for the configured game installation.

pub mod launch_config;
pub mod library;
pub mod locate;
pub mod vdf;

use std::path::PathBuf;

/// Resolved, read-only view of the game's location on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct SteamInfo {
    /// Steam app id.
    pub app_id: u32,
    /// Game installation root.
    pub game_dir: PathBuf,
    /// Directory holding `CrimsonDesert.exe` and ASI plugins.
    pub bin64_dir: PathBuf,
    /// Directory holding archive metadata such as `0.papgt`.
    pub meta_dir: PathBuf,
    /// Proton prefix (`steamapps/compatdata/<app_id>/pfx`), if derivable.
    pub prefix_dir: Option<PathBuf>,
}

/// Health of the configured installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    /// Executable and metadata were found.
    Ready,
    /// The directory exists but does not look like *Crimson Desert*.
    Unrecognised,
    /// The directory does not exist.
    Missing,
}

/// Which ASI loader proxy DLL (if any) is present in `bin64/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsiLoader {
    /// Ultimate ASI Loader installed as `winmm.dll`.
    Winmm,
    /// Ultimate ASI Loader installed as `version.dll`.
    Version,
    /// Ultimate ASI Loader installed as `dinput8.dll`.
    Dinput8,
    /// No known loader — ASI plugins will not be loaded.
    Absent,
}
