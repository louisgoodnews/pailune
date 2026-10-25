//! Job implementations. Each runs on a pool thread with its own DB connection.

mod analyze;
mod backups;
mod deploy;
mod nexus;
mod staging;

use super::context::JobContext;
use super::{JobKind, JobOutput};
use crate::error::{PyluneError, Result};
use crate::launch::{GameProcess, run};
use crate::steam::{GameStatus, SteamInfo, locate};

/// Dispatches a job to its implementation.
pub fn run(ctx: &JobContext, kind: JobKind) -> Result<JobOutput> {
    match kind {
        JobKind::Import { source, replace } => staging::import(ctx, &source, replace),
        JobKind::Download(job) => nexus::download(ctx, &job),
        JobKind::Apply => deploy::apply(ctx),
        JobKind::UnlinkAll => deploy::unlink_all(ctx),
        JobKind::CleanSlate => deploy::clean_slate(ctx),
        JobKind::RestoreOverlays => deploy::restore_overlays(ctx),
        JobKind::RefreshBackup => deploy::refresh_backup(ctx),
        JobKind::Remove(name) => staging::remove(ctx, &name),
        JobKind::RestoreDeleted(id) => staging::restore_deleted(ctx, &id),
        JobKind::PurgeDeleted(id) => staging::purge_deleted(ctx, &id),
        JobKind::CreateBackup { full, label } => backups::create(ctx, full, &label),
        JobKind::RestoreBackup(path) => backups::restore(ctx, &path),
        JobKind::DeleteBackup(path) => backups::delete(&path),
        JobKind::ValidateKey => nexus::validate(ctx),
        JobKind::FetchMod(id) => nexus::fetch_mod(ctx, id),
        JobKind::DescribeFile { mod_id, file_id } => nexus::describe_file(ctx, mod_id, file_id),
        JobKind::FetchCollection { slug, revision } => {
            nexus::fetch_collection(ctx, &slug, revision)
        }
        JobKind::CheckUpdates => nexus::check_updates(ctx),
        JobKind::AnalyzeConflicts(generation) => analyze::conflicts(ctx, generation),
    }
}

/// Fails unless the game directory is valid and the game is not running.
fn require_game(info: &SteamInfo) -> Result<()> {
    match (locate::game_status(info), run::game_process()) {
        (_, GameProcess::Running) => Err(PyluneError::Invalid(
            "Crimson Desert is running — close it first".into(),
        )),
        (GameStatus::Ready, GameProcess::Stopped) => Ok(()),
        (GameStatus::Unrecognised, _) => Err(PyluneError::Invalid(
            "game directory does not contain bin64/CrimsonDesert.exe and meta/0.papgt".into(),
        )),
        (GameStatus::Missing, _) => Err(PyluneError::Invalid("game directory not found".into())),
    }
}
