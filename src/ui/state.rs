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
    /// Text filter of the mod list (case-insensitive name match).
    pub filter: String,
    /// Deleted mods in the trash, newest first.
    pub trash: Vec<crate::mod_engine::trash::TrashItem>,
    /// Trash id of the mod deleted last, offered as "Undo".
    pub undo: Option<String>,
    /// An imported mod list whose missing mods are still being installed;
    /// re-applied after each finished download.
    pub list_import: Option<ListImport>,
    /// Dialog state when creating a new JSON mod.
    pub create_mod: Option<CreateModDraft>,
    /// Dialog state when packaging a mod for distribution.
    pub package_mod: Option<crate::mod_engine::package::PackageMetadata>,
    /// Dialog state when spawning a new mod project via the SDK.
    pub spawn_project: Option<SpawnProjectDraft>,
}

/// Draft state for spawning a new mod project via the SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnProjectDraft {
    pub name: String,
    pub mod_type: crate::sdk::ProjectType,
    pub location: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub stage_after: bool,
}

impl Default for SpawnProjectDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            mod_type: crate::sdk::ProjectType::Asi,
            location: String::new(),
            author: String::new(),
            version: "1.0".into(),
            description: String::new(),
            stage_after: true,
        }
    }
}

/// Draft fields for creating a new JSON mod in staging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateModDraft {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
}

impl Default for CreateModDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            author: String::new(),
            version: "1.0".into(),
            description: String::new(),
        }
    }
}

/// An imported mod list and the mods it names that are not installed.
#[derive(Debug, Clone)]
pub struct ListImport {
    pub list: crate::modlist::ModList,
    pub missing: Vec<crate::modlist::ListedMod>,
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

/// A pending operation on a dynamic list item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListOp {
    Delete {
        list_at: usize,
        index: usize,
        label: String,
    },
    Duplicate {
        list_at: usize,
        index: usize,
        label: String,
    },
}

/// An open save in the viewer and the values typed into it.
#[derive(Debug, Clone, Default)]
pub struct SaveView {
    /// Slot folder name, e.g. `slot0`.
    pub slot: String,
    pub dir: std::path::PathBuf,
    pub tree: Vec<crate::savefile::view::Node>,
    pub query: String,
    /// Typed text per body offset, while it differs from the save.
    pub edits: std::collections::BTreeMap<usize, String>,
    /// Pending list item operations.
    pub list_ops: Vec<ListOp>,
}

/// Requests only the window can carry out, collected while dispatching
/// and executed at the end of the frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowRequests {
    /// Close Pailune (through the normal close, so its guard applies).
    pub quit: bool,
    /// Text to put on the clipboard.
    pub clipboard: Option<String>,
}

/// Log page state.
#[derive(Debug, Clone, Default)]
pub struct LogState {
    pub events: Vec<Event>,
    /// Show only events of the selected mod.
    pub only_selected: bool,
}

/// Data Browser page state.
#[derive(Debug, Clone, Default)]
pub struct DataBrowserState {
    pub tables: Vec<crate::tables::browser::TableSummary>,
    pub table_filter: String,
    pub selected_table: Option<String>,
    pub record_filter: String,
    pub records: Vec<crate::tables::Record>,
    pub body: Vec<u8>,
    pub selected_record: Option<u64>,
    pub inspected: Option<crate::tables::browser::InspectedRecord>,
    pub error: Option<String>,
    pub loaded: bool,
    /// Destination mod for injecting edits from the browser.
    pub target_mod: Option<String>,
    /// Field edit draft currently open in the inspector.
    pub active_edit: Option<ActiveFieldEdit>,
}

/// An edit draft being prepared in the Data Browser inspector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFieldEdit {
    pub field_name: String,
    pub field_type: String,
    pub original_value: String,
    pub new_value: String,
    pub is_raw: bool,
    pub rel_offset: usize,
    pub original_hex: String,
}

/// Visual node-based mod creator state.
#[derive(Debug, Clone)]
pub struct CreatorState {
    pub graph: crate::visual::Graph,
    pub pan: [f32; 2],
    pub zoom: f32,
    pub selected_node: Option<String>,
    pub dragging_node: Option<String>,
    pub connecting_socket: Option<(String, String, bool)>,
    pub show_diff_preview: bool,
    pub status_feedback: Option<(String, super::Level)>,
}

impl Default for CreatorState {
    fn default() -> Self {
        Self {
            graph: crate::visual::templates::mega_stacks(999),
            pan: [0.0, 0.0],
            zoom: 1.0,
            selected_node: None,
            dragging_node: None,
            connecting_socket: None,
            show_diff_preview: true,
            status_feedback: None,
        }
    }
}
