//! Per-page UI state. Plain data; behaviour lives in [`super::actions`].

use crate::jobs::{JobId, JobKind};
use crate::logging::Event;
use crate::mod_engine::conflicts::ConflictReport;
use crate::mod_engine::model::ModDetails;
use crate::nexus::collection::Collection;
use crate::nexus::{FileSummary, NexusFile, NexusMod, NexusUser, NxmLink};
use crate::profiles::ProfileBook;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

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
    pub kind: JobKind,
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
    /// The mod list changed while an analysis was in flight; run again after it.
    pub conflicts_rerun: bool,
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
    /// `nxm://` downloads waiting for the user's confirmation.
    pub incoming: Vec<Incoming>,
    /// Collection slug or URL being typed.
    pub collection_draft: String,
    /// A fetched collection under review, with the user's member choices.
    pub preview: Option<Collection>,
    /// Installed collections (from the database).
    pub collections: Vec<Collection>,
}

/// An `nxm://` download request from the browser, not yet confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    pub link: NxmLink,
    /// Names looked up from Nexus; `None` while the lookup runs.
    pub summary: Option<FileSummary>,
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
