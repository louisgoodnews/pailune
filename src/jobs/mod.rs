//! Background jobs executed in parallel on a worker pool.
//!
//! Jobs that touch the game directory or remove staged mods are
//! [`Access::Exclusive`]: they run one at a time. Everything else (imports,
//! downloads, Nexus queries, conflict analysis) runs concurrently.

pub mod context;
pub mod pool;
pub mod tasks;

use crate::mod_engine::conflicts::ConflictReport;
use crate::nexus::collection::Collection;
use crate::nexus::download::DownloadJob;
use crate::nexus::{FileSummary, NexusFile, NexusMod, NexusUser};
use std::path::PathBuf;

/// Identifies a submitted job.
pub type JobId = u64;

/// Every kind of background work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobKind {
    /// Stage a file, archive or folder; `replace` updates an existing mod.
    Import {
        source: PathBuf,
        replace: Option<String>,
    },
    /// Download from Nexus and stage.
    Download(DownloadJob),
    Apply,
    UnlinkAll,
    CleanSlate,
    RestoreOverlays,
    RefreshBackup,
    /// Delete a staged mod (and redeploy if it was linked).
    Remove(String),
    ValidateKey,
    FetchMod(u64),
    /// Fetch a collection revision (latest when `revision` is `None`).
    FetchCollection {
        slug: String,
        revision: Option<u32>,
    },
    /// Look up names for an incoming `nxm://` download.
    DescribeFile {
        mod_id: u64,
        file_id: u64,
    },
    CheckUpdates,
    /// Conflict analysis; the generation lets the UI drop stale results.
    AnalyzeConflicts(u64),
}

/// How a job synchronises with others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Shared,
    Exclusive,
}

/// Successful job results.
#[derive(Debug, Clone, PartialEq)]
pub enum JobOutput {
    /// Generic completion message.
    Done(String),
    User(NexusUser),
    Mod(NexusMod, Vec<NexusFile>),
    /// `(mod name, newer file)` for every outdated mod, plus failed checks.
    Updates {
        outdated: Vec<(String, String)>,
        failed: usize,
    },
    Described(FileSummary),
    Collection(Collection),
    Conflicts(u64, ConflictReport),
}

/// Lifecycle notifications sent to the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum JobUpdate {
    Started,
    /// Bytes or items done, with an optional total.
    Progress {
        done: u64,
        total: Option<u64>,
    },
    Finished(Result<JobOutput, String>),
}

/// One notification about one job.
#[derive(Debug, Clone, PartialEq)]
pub struct JobEvent {
    pub id: JobId,
    pub update: JobUpdate,
}

impl JobKind {
    /// Whether the job must run alone.
    pub fn access(&self) -> Access {
        match self {
            JobKind::Apply
            | JobKind::UnlinkAll
            | JobKind::CleanSlate
            | JobKind::RestoreOverlays
            | JobKind::RefreshBackup
            | JobKind::Remove(_) => Access::Exclusive,
            _ => Access::Shared,
        }
    }

    /// Short human-readable title for the jobs panel.
    pub fn title(&self) -> String {
        match self {
            JobKind::Import {
                source,
                replace: None,
            } => format!("Import {}", file_name(source)),
            JobKind::Import {
                replace: Some(name),
                ..
            } => format!("Update {name}"),
            JobKind::Download(job) => format!("Download Nexus file {}", job.file_id),
            JobKind::Apply => "Apply mods".into(),
            JobKind::UnlinkAll => "Unlink all".into(),
            JobKind::CleanSlate => "Clean slate".into(),
            JobKind::RestoreOverlays => "Restore overlays".into(),
            JobKind::RefreshBackup => "Refresh backup".into(),
            JobKind::Remove(name) => format!("Delete {name}"),
            JobKind::ValidateKey => "Validate Nexus key".into(),
            JobKind::FetchMod(id) => format!("Fetch Nexus mod {id}"),
            JobKind::DescribeFile { file_id, .. } => format!("Look up Nexus file {file_id}"),
            JobKind::FetchCollection { slug, .. } => format!("Fetch collection {slug}"),
            JobKind::CheckUpdates => "Check updates".into(),
            JobKind::AnalyzeConflicts(_) => "Analyze conflicts".into(),
        }
    }
}

/// File name of a path for display, falling back to the whole path.
fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}
