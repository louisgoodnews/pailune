//! Per-job environment handed to task implementations.

use super::{JobEvent, JobId, JobUpdate};
use crate::config::Config;
use crate::config::paths::DataPaths;
use crate::db;
use crate::error::Result;
use crate::logging::{Event, journal};
use crate::steam::{SteamInfo, locate};
use rusqlite::Connection;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};

/// Everything a running job may use.
pub struct JobContext {
    pub id: JobId,
    pub paths: DataPaths,
    /// Configuration snapshot taken when the job was submitted.
    pub config: Config,
    pub(super) events: Sender<JobEvent>,
    pub(super) notify: Arc<dyn Fn() + Send + Sync>,
    pub(super) game_lock: Arc<Mutex<()>>,
}

impl JobContext {
    /// Opens a fresh database connection for this job's thread.
    pub fn db(&self) -> Result<Connection> {
        db::open(&self.paths.database)
    }

    /// Resolved game paths for the job's configuration.
    pub fn steam(&self) -> SteamInfo {
        locate::resolve(&self.config)
    }

    /// Reports progress to the UI.
    pub fn progress(&self, done: u64, total: Option<u64>) {
        self.send(JobUpdate::Progress { done, total });
    }

    /// Sends a lifecycle update for this job and wakes the UI.
    pub(super) fn send(&self, update: JobUpdate) {
        let _ = self.events.send(JobEvent {
            id: self.id,
            update,
        });
        (self.notify)();
    }

    /// Serialises access to the game directory and staged-mod replacement.
    /// A poisoned lock (panicked job) is recovered: the data it guards is the
    /// filesystem, which the manifest keeps consistent.
    pub fn lock_game(&self) -> MutexGuard<'_, ()> {
        self.game_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Records an event in the journal.
    pub fn journal(&self, conn: &Connection, event: Event) {
        journal::record(conn, &self.paths.mod_logs, &event);
    }
}
