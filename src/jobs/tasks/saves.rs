//! Save-game backup jobs.

use crate::db;
use crate::error::{PyluneError, Result};
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::logging::{Event, EventKind, Severity};
use crate::save_backup;
use crate::steam::locate;
use std::path::{Path, PathBuf};

/// The save folder of the configured game's Proton prefix.
pub fn save_folder(ctx: &JobContext) -> Result<PathBuf> {
    save_backup::save_dir(locate::resolve(&ctx.config).prefix_dir.as_deref()).ok_or_else(|| {
        PyluneError::Invalid(
            "the game's Proton prefix was not found — start the game once from Steam".into(),
        )
    })
}

/// Records a save-backup event in the journal (best effort).
fn journal(ctx: &JobContext, text: String) {
    if let Ok(conn) = ctx.db() {
        ctx.journal(
            &conn,
            Event::new(Severity::Info, EventKind::Backup, None, text),
        );
    }
}

/// Backs up the save folder on request.
pub fn backup(ctx: &JobContext) -> Result<JobOutput> {
    let saves = save_folder(ctx)?;
    let info = save_backup::create(&saves, &ctx.paths.save_backups, "manual", db::now())?;
    let text = format!("Save games backed up ({} file(s))", info.files);
    journal(ctx, format!("{text}: {}", info.path.display()));
    Ok(JobOutput::Done(text))
}

/// Puts a save backup back in place (current saves are backed up first).
pub fn restore(ctx: &JobContext, archive: &Path) -> Result<JobOutput> {
    let saves = save_folder(ctx)?;
    save_backup::restore(archive, &saves, &ctx.paths.save_backups, db::now())?;
    let text = "Save games restored — if Steam reports a cloud conflict, keep the local files";
    journal(ctx, format!("{text}: {}", archive.display()));
    Ok(JobOutput::Done(text.into()))
}
