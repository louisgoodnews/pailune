//! Operations shared by the command line and the MCP server (B-14): one
//! enum, so both front-ends offer exactly the same things.

use std::path::PathBuf;

/// Everything Pailune can do without its window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Game, deployment and mod counts.
    Status,
    /// Mods in load order.
    List,
    Profiles,
    /// Recent journal events, optionally of one mod.
    Log {
        mod_name: Option<String>,
        limit: usize,
    },
    /// Pailune and save-game backups.
    Backups,
    /// The game's save slots.
    Saves,
    /// The contents of one save slot (read-only), optionally filtered.
    SaveShow {
        slot: String,
        find: Option<String>,
    },
    SetEnabled {
        names: Vec<String>,
        enabled: bool,
    },
    SetAll(bool),
    /// Move to a 1-based load-order number.
    Move {
        name: String,
        position: usize,
    },
    SetVariant {
        name: String,
        variant: String,
    },
    Install {
        paths: Vec<PathBuf>,
    },
    /// Move a mod to the trash (restorable).
    Delete {
        name: String,
    },
    Apply,
    Unlink,
    CleanSlate,
    ProfileSave {
        name: String,
    },
    ProfileLoad {
        name: String,
    },
    ExportList {
        path: PathBuf,
    },
    ImportList {
        path: PathBuf,
    },
    Backup {
        full: bool,
    },
    RestoreBackup {
        path: PathBuf,
    },
    BackupSaves,
    CheckUpdates,
    Launch,
}

impl Op {
    /// Reads state only; works while the window is open.
    pub fn read_only(&self) -> bool {
        matches!(
            self,
            Op::Status
                | Op::List
                | Op::Profiles
                | Op::Log { .. }
                | Op::Backups
                | Op::Saves
                | Op::SaveShow { .. }
        )
    }

    /// Cannot be undone from Pailune; front-ends ask for explicit confirmation.
    pub fn destructive(&self) -> bool {
        matches!(self, Op::CleanSlate | Op::RestoreBackup { .. })
    }
}

/// Result of an operation: a sentence for people and data for programs.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub text: String,
    pub data: serde_json::Value,
}

impl Report {
    /// A report without data.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            data: serde_json::Value::Null,
        }
    }
}
