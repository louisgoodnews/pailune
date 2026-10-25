//! egui front-end. [`PyluneApp`] holds plain state; behaviour lives in
//! [`actions`] (state transitions) and [`views`] (rendering).

mod actions;
mod app;
mod draft;
mod theme;
mod views;

use crate::backup::BackupStatus;
use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::jobs::JobId;
use crate::jobs::pool::JobPool;
use crate::launch::GameProcess;
use crate::logging::Event;
use crate::mod_engine::conflicts::ConflictReport;
use crate::mod_engine::model::{ModDetails, MoveTo, Registry};
use crate::nexus::{NexusFile, NexusMod, NexusUser};
use crate::profiles::ProfileBook;
use crate::steam::{AsiLoader, GameStatus, SteamInfo};
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::PathBuf;
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
    pub default_options: String,
    pub max_parallel: String,
    pub log_level: String,
}

/// Main-area page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Mods,
    Profiles,
    Nexus,
    Log,
    Settings,
}

/// Destructive actions that need an explicit confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirm {
    CleanSlate,
    DeleteMod(String),
    DeleteProfile(String),
}

/// Mod management intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModAction {
    Select(String),
    SetEnabled(String, bool),
    SetAll(bool),
    Move(String, MoveTo),
    Delete(String),
    /// Nexus mods: pick a newer file; others: pick a replacement archive.
    Update(String),
    OpenFolder(String),
    ShowReadme(PathBuf),
    CloseReadme,
    ImportFolder,
    ImportFiles,
    Rescan,
}

/// Game-directory intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployAction {
    Apply,
    UnlinkAll,
    CleanSlate,
    RestoreOverlays,
    RefreshBackup,
    Launch,
}

/// Profile page intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileAction {
    /// Save the current selection under this name (new or overwrite).
    Save(String),
    Load(String),
    Delete(String),
    /// Store the edited launch options of this profile.
    SaveOptions(String),
}

/// Nexus page intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NexusAction {
    SaveKey,
    Validate,
    Fetch,
    Download(u64),
    CheckUpdates,
    OpenPage,
    RegisterHandler,
}

/// Every user intent the views can emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Mod(ModAction),
    Deploy(DeployAction),
    Profile(ProfileAction),
    Nexus(NexusAction),
    SwitchTab(Tab),
    BrowseGameDir,
    SaveSettings,
    RefreshLog,
    OpenLogFolder,
    Ask(Confirm),
    Dismiss,
}

/// Lifecycle of a job as shown in the jobs panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
}

/// One row of the jobs panel.
#[derive(Debug, Clone, PartialEq)]
pub struct JobView {
    pub id: JobId,
    pub kind: crate::jobs::JobKind,
    pub title: String,
    pub state: JobState,
    pub progress: Option<(u64, Option<u64>)>,
    pub finished_at: Option<Instant>,
}

/// Mods page state.
#[derive(Debug, Clone, Default)]
pub struct ModsState {
    pub selected: Option<String>,
    /// Details of the selected mod (name, details).
    pub details: Option<(String, ModDetails)>,
    /// Open readme (path, text).
    pub readme: Option<(PathBuf, String)>,
    pub conflicts: ConflictReport,
    /// Generation of the newest requested analysis.
    pub conflict_generation: u64,
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
    /// Staged mod name → newer version available on Nexus.
    pub updates: BTreeMap<String, String>,
    pub handler_registered: bool,
}

/// Profile page state.
#[derive(Debug, Clone, Default)]
pub struct ProfilesState {
    pub book: ProfileBook,
    pub name_draft: String,
    /// Profile name → launch options being edited.
    pub option_drafts: BTreeMap<String, String>,
}

/// Log page state.
#[derive(Debug, Clone, Default)]
pub struct LogState {
    pub events: Vec<Event>,
    /// Show only events of the selected mod.
    pub only_selected: bool,
}

/// Complete application state.
pub struct PyluneApp {
    pub paths: DataPaths,
    pub config: Config,
    pub draft: SettingsDraft,
    /// The UI thread's own database connection.
    pub conn: Connection,
    pub registry: Registry,
    pub deployed_links: usize,
    pub info: SteamInfo,
    pub game: GameStatus,
    pub loader: AsiLoader,
    pub backup: BackupStatus,
    pub status: StatusLine,
    pub confirm: Option<Confirm>,
    /// The game directory lags behind the mod list (derived from the DB).
    pub dirty: bool,
    pub tab: Tab,
    pub process: GameProcess,
    pub mods: ModsState,
    pub profiles: ProfilesState,
    pub nexus: NexusState,
    pub log: LogState,
    pub pool: JobPool,
    pub jobs: Vec<JobView>,
    /// Apply job after which the game should be launched.
    pub launch_after: Option<JobId>,
    /// Last time the game process and nxm queue were polled.
    pub last_poll: Instant,
    /// A close request was refused because a game-folder job is running;
    /// the next request closes anyway.
    pub close_refused: bool,
    /// A second start asked this window to come to the front.
    pub focus_requested: bool,
}
