//! Backup jobs (exclusive: the state must not change while it is copied).

use crate::app_backup::{AUTO_LABEL, BackupKind, create, restore as restore_backup};
use crate::db;
use crate::error::{IoContext, PyluneError, Result};
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::logging::{Event, EventKind, Severity};
use std::fs;
use std::path::Path;

/// Writes a backup; automatic ones are pruned to the newest few.
pub fn create(ctx: &JobContext, full: bool, label: &str) -> Result<JobOutput> {
    let conn = ctx.db()?;
    let kind = if full {
        BackupKind::Full
    } else {
        BackupKind::Settings
    };
    let info = create::create(&ctx.paths, &conn, kind, label, db::now())?;
    if label == AUTO_LABEL {
        create::prune_auto(&ctx.paths.backups, ctx.config.backup.keep_auto.max(1))?;
    }
    let text = format!(
        "Backup saved ({} mod(s), {:.1} MB)",
        info.mods,
        info.size as f64 / 1e6
    );
    ctx.journal(
        &conn,
        Event::new(
            Severity::Info,
            EventKind::Backup,
            None,
            format!("{text}: {}", info.path.display()),
        ),
    );
    Ok(JobOutput::Done(text))
}

/// Restores a backup after taking a safety backup of the current state.
pub fn restore(ctx: &JobContext, archive: &Path) -> Result<JobOutput> {
    let mut conn = ctx.db()?;
    if !db::links::load(&conn)?.links.is_empty() {
        return Err(PyluneError::Invalid(
            "unlink all mods before restoring a backup".into(),
        ));
    }
    create::create(
        &ctx.paths,
        &conn,
        BackupKind::Settings,
        "before restore",
        db::now(),
    )?;
    let report = restore_backup::restore(&ctx.paths, &mut conn, archive, db::now())?;
    let parts = [
        (true, "mod list and profiles"),
        (report.settings, "settings"),
        (report.mod_files, "mod files"),
    ];
    let restored: Vec<&str> = parts
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, what)| *what)
        .collect();
    let text = format!("Restored {} — review and press Apply", restored.join(", "));
    ctx.journal(
        &conn,
        Event::new(
            Severity::Info,
            EventKind::Backup,
            None,
            format!("{text} from {}", archive.display()),
        ),
    );
    Ok(JobOutput::Done(text))
}

/// Deletes a backup file.
pub fn delete(path: &Path) -> Result<JobOutput> {
    fs::remove_file(path).at(path)?;
    Ok(JobOutput::Done("Backup deleted".into()))
}
