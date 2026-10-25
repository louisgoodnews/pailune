//! egui front-end. [`PyluneApp`] holds plain state; behaviour lives in
//! [`actions`] (state transitions) and [`views`] (rendering).

mod actions;
mod app;
mod draft;
mod theme;
mod views;

use crate::backup::BackupStatus;
use crate::config::Config;
use crate::config::store::DataPaths;
use crate::mod_engine::model::Registry;
use crate::steam::{AsiLoader, GameStatus, SteamInfo};
use crate::symlink::Manifest;

/// Severity of the message in the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Success,
    Warning,
    Error,
}

/// The single line of feedback shown at the bottom of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    pub level: Level,
    pub text: String,
}

/// Editable copy of the settings, committed with "Save".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsDraft {
    pub game_dir: String,
    pub app_id: String,
    pub json_subdir: String,
}

/// Destructive actions that need an explicit confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirm {
    CleanSlate,
    Remove(usize),
}

/// Every user intent the views can emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Toggle(usize),
    Move { from: usize, to: usize },
    Remove(usize),
    ImportFolder,
    ImportFile,
    Apply,
    UnlinkAll,
    CleanSlate,
    RestoreOverlays,
    RefreshBackup,
    BrowseGameDir,
    SaveSettings,
    Rescan,
    Ask(Confirm),
    Dismiss,
}

/// Complete application state.
pub struct PyluneApp {
    pub paths: DataPaths,
    pub config: Config,
    pub draft: SettingsDraft,
    pub registry: Registry,
    pub manifest: Manifest,
    pub info: SteamInfo,
    pub game: GameStatus,
    pub loader: AsiLoader,
    pub backup: BackupStatus,
    pub status: StatusLine,
    pub confirm: Option<Confirm>,
    /// The mod selection changed since the last apply.
    pub dirty: bool,
}
