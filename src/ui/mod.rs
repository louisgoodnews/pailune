//! egui front-end. [`PailuneApp`] holds plain state; behaviour lives in
//! [`actions`] (state transitions) and [`views`] (rendering).

mod actions;
mod app;
mod draft;
pub mod fatal;
pub mod guide;
pub mod icon;
mod intent;
pub mod notify;
mod shortcuts;
mod startup;
mod state;
mod theme;
mod views;

#[allow(unused_imports)]
pub use intent::{
    Action, BackupAction, Confirm, CreatorAction, DataBrowserAction, DeployAction, ModAction, ModListAction,
    NexusAction, ProfileAction, SetupAction, Tab,
};
#[allow(unused_imports)]
pub use state::{
    ActiveFieldEdit, CreatorState, DataBrowserState, Incoming, JobState, JobView, LogState, ModsState,
    NexusState, ProfilesState, SpawnProjectDraft,
};

use crate::backup::BackupStatus;
use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::jobs::JobId;
use crate::jobs::pool::JobPool;
use crate::launch::GameProcess;
use crate::mod_engine::model::Registry;
use crate::steam::{AsiLoader, GameStatus, SteamInfo};
use rusqlite::Connection;
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

/// Category shown on the settings page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Appearance,
    Behavior,
    Mods,
    Game,
    Nexus,
    Backups,
    Advanced,
    Diagnostics,
}

/// Editable copy of the settings, committed with "Save". Values are edited
/// in place with constrained widgets; only the game folder is free text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SettingsDraft {
    pub game_dir: String,
    pub config: Config,
    pub page: SettingsPage,
}

/// Complete application state.
pub struct PailuneApp {
    pub paths: DataPaths,
    pub config: Config,
    pub draft: SettingsDraft,
    /// Appearance currently installed on the egui context.
    pub applied_ui: crate::config::UiConfig,
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
    pub data_browser: DataBrowserState,
    pub creator: CreatorState,
    /// An ASI loader is installed but Steam's launch options leave it off.
    pub loader_off: bool,
    /// Toasts and the notification center.
    pub notices: notify::Notifications,
    /// Window commands requested by this frame's actions.
    pub window: state::WindowRequests,
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
    /// The first-run dialog for choosing the game folder is open.
    pub setup_open: bool,
    /// Backups of Pailune's state, newest first.
    pub backups: Vec<crate::app_backup::BackupInfo>,
    /// Backups of the game's save files, newest first.
    pub save_backups: Vec<crate::save_backup::SaveBackup>,
    /// Running without a window (command line, MCP): requests from other
    /// instances are left in the spool for the window.
    pub headless: bool,
    /// Failure messages of jobs, in order (read by the headless engine).
    pub job_failures: Vec<String>,
    /// ASI plugins in the game folder.
    pub plugins: Vec<crate::mod_engine::plugins::Plugin>,
    /// The game's save slots (summaries).
    pub save_slots: Vec<crate::savefile::slots::Slot>,
    /// An open save viewer: slot name, tree, search text.
    pub save_view: Option<state::SaveView>,
    /// Onboarding window.
    pub guide: guide::GuideState,
    /// Version and pack groups of the installed archives.
    pub archives: crate::paz::status::ArchiveStatus,
}
