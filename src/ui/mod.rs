//! egui front-end. [`PyluneApp`] holds plain state; behaviour lives in
//! [`actions`] (state transitions) and [`views`] (rendering).

mod actions;
mod app;
mod draft;
pub mod fatal;
pub mod icon;
mod shortcuts;
mod state;
mod theme;
mod views;

pub use state::{Incoming, JobState, JobView, LogState, ModsState, NexusState, ProfilesState};

use crate::backup::BackupStatus;
use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::jobs::JobId;
use crate::jobs::pool::JobPool;
use crate::launch::GameProcess;
use crate::mod_engine::model::{MoveTo, Registry};
use crate::steam::{AsiLoader, GameStatus, SteamInfo};
use rusqlite::Connection;
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
    UninstallCollection(String),
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
    /// Files or folders dropped onto the window.
    ImportPaths(Vec<PathBuf>),
    /// Choose the option folder of a multi-variant mod.
    SetVariant(String, String),
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
    /// Ask Steam to verify the game files (after a game update).
    VerifyGame,
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
    /// Write a profile to a JSON file chosen by the user.
    Export(String),
    /// Read a profile from a JSON file.
    Import,
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
    /// Start the confirmed incoming download at this index.
    AcceptIncoming(usize),
    /// Drop the incoming download at this index.
    RejectIncoming(usize),
    /// Look up the collection in the draft field.
    FetchCollection,
    /// Flip whether an optional member of the previewed collection is installed.
    ToggleMember(usize),
    /// Install (or update to) the previewed collection.
    InstallCollection,
    ClosePreview,
    /// Fetch the newest revision of an installed collection for review.
    UpdateCollection(String),
    /// Enable or disable every installed member.
    CollectionEnabled(String, bool),
    UninstallCollection(String),
    /// Open a member's file page on Nexus (free accounts download there).
    OpenMemberPage(u64, u64),
}

/// First-run setup dialog intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupAction {
    /// Search Steam's libraries again.
    Detect,
    /// Save the chosen folder and close the dialog if it is valid.
    Confirm,
    /// Close the dialog without a valid game folder.
    Later,
}

/// Every user intent the views can emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Mod(ModAction),
    Deploy(DeployAction),
    Profile(ProfileAction),
    Nexus(NexusAction),
    Setup(SetupAction),
    SwitchTab(Tab),
    BrowseGameDir,
    SaveSettings,
    RefreshLog,
    OpenLogFolder,
    Ask(Confirm),
    Dismiss,
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
    /// The first-run dialog for choosing the game folder is open.
    pub setup_open: bool,
    /// Version and pack groups of the installed archives.
    pub archives: crate::paz::status::ArchiveStatus,
}
