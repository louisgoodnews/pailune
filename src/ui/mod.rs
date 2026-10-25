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
use crate::launch::GameProcess;
use crate::mod_engine::model::Registry;
use crate::nexus::worker::WorkerHandle;
use crate::nexus::{NexusFile, NexusMeta, NexusMod, NexusUser};
use crate::profiles::ProfileBook;
use crate::steam::{AsiLoader, GameStatus, SteamInfo};
use crate::symlink::Manifest;
use std::collections::BTreeMap;
use std::time::Instant;

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
    pub nexus_domain: String,
    pub apply_before_launch: bool,
}

/// Main-area page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Mods,
    Profiles,
    Nexus,
}

/// Destructive actions that need an explicit confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirm {
    CleanSlate,
    Remove(usize),
    DeleteProfile(String),
}

/// Profile page intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileAction {
    /// Save the current selection under this name (new or overwrite).
    Save(String),
    Load(String),
    Delete(String),
}

/// Nexus page intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NexusAction {
    SaveKey,
    Validate,
    Fetch,
    Download(u64),
    CheckUpdates,
    /// Open the file list of a staged mod's Nexus page to update it.
    Update(String),
    OpenPage,
    RegisterHandler,
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
    Launch,
    SwitchTab(Tab),
    Profile(ProfileAction),
    Nexus(NexusAction),
    Ask(Confirm),
    Dismiss,
}

/// Nexus page state.
#[derive(Debug, Clone, Default)]
pub struct NexusState {
    pub key_draft: String,
    pub mod_id_draft: String,
    pub user: Option<NexusUser>,
    /// Mod page currently shown, with its files.
    pub browsing: Option<(NexusMod, Vec<NexusFile>)>,
    /// Staged mod that the next download from `browsing` replaces.
    pub replace_target: Option<String>,
    /// `(file, bytes done, total)` of the running download.
    pub progress: Option<(String, u64, Option<u64>)>,
    /// Requests sent but not yet answered.
    pub pending: usize,
    /// Staged mod name → newer version available on Nexus.
    pub updates: BTreeMap<String, String>,
    pub meta: NexusMeta,
    pub handler_registered: bool,
}

/// Profile page state.
#[derive(Debug, Clone, Default)]
pub struct ProfilesState {
    pub book: ProfileBook,
    pub name_draft: String,
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
    pub tab: Tab,
    pub process: GameProcess,
    pub profiles: ProfilesState,
    pub nexus: NexusState,
    pub worker: WorkerHandle,
    /// Last time the game process and nxm queue were polled.
    pub last_poll: Instant,
}
