//! Save-game backup jobs.

use crate::db;
use crate::error::{IoContext, PailuneError, Result};
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::launch::{GameProcess, run};
use crate::logging::{Event, EventKind, Severity};
use crate::save_backup;
use crate::savefile::{self, edit::SaveEdit};
use crate::steam::locate;
use std::fs;
use std::path::{Path, PathBuf};

/// The save folder of the configured game's Proton prefix.
pub fn save_folder(ctx: &JobContext) -> Result<PathBuf> {
    save_backup::save_dir(locate::resolve(&ctx.config).prefix_dir.as_deref()).ok_or_else(|| {
        PailuneError::Invalid(
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

/// Changes values of one slot's `save.save` (B-07). Refused while the game
/// runs; every save is backed up first; the file is replaced atomically and
/// only after the rewritten save read back exactly as intended.
pub fn edit(ctx: &JobContext, slot: &Path, edits: &[SaveEdit]) -> Result<JobOutput> {
    if run::game_process() == GameProcess::Running {
        return Err(PailuneError::Invalid(
            "close Crimson Desert before editing a save — it would overwrite the change".into(),
        ));
    }
    let saves = save_folder(ctx)?;
    if !slot.starts_with(&saves) {
        return Err(PailuneError::Invalid(
            "not a slot of the game's save folder".into(),
        ));
    }
    let path = slot.join("save.save");
    let written = savefile::edit::apply(&fs::read(&path).at(&path)?, edits)?;
    let backup = save_backup::create(&saves, &ctx.paths.save_backups, "before-edit", db::now())?;
    let temp = slot.join(".save.save.pailune-edit");
    fs::write(&temp, &written).at(&temp)?;
    fs::rename(&temp, &path).at(&path)?;
    let slot_name = slot.file_name().unwrap_or_default().to_string_lossy();
    let changes: Vec<&str> = edits.iter().map(|e| e.label.as_str()).collect();
    let text = format!("Saved {} change(s) to {slot_name}", edits.len());
    journal(
        ctx,
        format!(
            "{text}: {} (backup {})",
            changes.join(", "),
            backup.path.display()
        ),
    );
    Ok(JobOutput::Done(format!(
        "{text} — all saves were backed up first; if Steam reports a cloud conflict, keep the local files"
    )))
}
