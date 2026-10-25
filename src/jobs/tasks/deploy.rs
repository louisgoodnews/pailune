//! Exclusive jobs touching the game directory.

use super::require_game;
use crate::backup::{reset, snapshot};
use crate::db;
use crate::db::links::DbJournal;
use crate::error::{PyluneError, Result};
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::logging::{Event, EventKind, Severity};
use crate::mod_engine::{model::ModEntry, plan};
use crate::symlink::{Report, deploy, undeploy};
use rusqlite::Connection;

/// Turns a report into a message, logging any warnings.
fn summarise(
    ctx: &JobContext,
    conn: &Connection,
    kind: EventKind,
    verb: &str,
    report: &Report,
) -> String {
    report
        .warnings
        .iter()
        .for_each(|w| ctx.journal(conn, Event::new(Severity::Warn, kind, None, w.clone())));
    let text = format!(
        "{verb}: {} link(s), {} backup(s) touched",
        report.linked, report.backed_up
    );
    match report.warnings.len() {
        0 => text,
        n => format!("{text}; {n} warning(s), see the Log tab"),
    }
}

/// Unlinks everything and links every enabled mod; shared by apply and remove.
pub(super) fn redeploy(ctx: &JobContext, conn: &mut Connection) -> Result<String> {
    let info = ctx.steam();
    require_game(&info)?;
    let mut manifest = db::links::load(conn)?;
    let mut journal = DbJournal { conn };
    undeploy::undeploy(&mut manifest, &mut journal)?;
    snapshot::ensure_meta_backup(&info)?;
    let (registry, revision) = db::mods::load_with_revision(conn)?;
    let link_plan = plan::build(&ctx.config, &ctx.paths.staging, &registry)?;
    let report = deploy::deploy(&link_plan, &ctx.paths.staging, &mut manifest, &mut journal)?;
    db::mods::set_deployed_revision(conn, Some(revision))?;
    let enabled: Vec<&ModEntry> = registry.mods.iter().filter(|m| m.enabled).collect();
    enabled.iter().for_each(|m| {
        let msg = format!(
            "deployed ({} file(s), load position {})",
            m.file_count,
            position(&registry.mods, &m.name)
        );
        ctx.journal(
            conn,
            Event::new(Severity::Info, EventKind::Deploy, Some(&m.name), msg),
        );
    });
    let mut text = summarise(ctx, conn, EventKind::Deploy, "Applied", &report);
    if link_plan.conflicts > 0 {
        text.push_str(&format!(
            " — {} file conflict(s), later mods won",
            link_plan.conflicts
        ));
    }
    ctx.journal(
        conn,
        Event::new(Severity::Info, EventKind::Deploy, None, text.clone()),
    );
    Ok(text)
}

fn position(mods: &[ModEntry], name: &str) -> usize {
    mods.iter()
        .position(|m| m.name == name)
        .map_or(0, |i| i + 1)
}

pub fn apply(ctx: &JobContext) -> Result<JobOutput> {
    let mut conn = ctx.db()?;
    redeploy(ctx, &mut conn).map(JobOutput::Done)
}

pub fn unlink_all(ctx: &JobContext) -> Result<JobOutput> {
    let conn = ctx.db()?;
    require_game(&ctx.steam())?;
    let mut manifest = db::links::load(&conn)?;
    let report = undeploy::undeploy(&mut manifest, &mut DbJournal { conn: &conn })?;
    db::mods::set_deployed_revision(&conn, None)?;
    let text = summarise(ctx, &conn, EventKind::Unlink, "Unlinked", &report);
    ctx.journal(
        &conn,
        Event::new(Severity::Info, EventKind::Unlink, None, text.clone()),
    );
    Ok(JobOutput::Done(text))
}

pub fn clean_slate(ctx: &JobContext) -> Result<JobOutput> {
    let mut conn = ctx.db()?;
    let info = ctx.steam();
    require_game(&info)?;
    let mut manifest = db::links::load(&conn)?;
    let report = reset::clean_slate(&info, &mut manifest, &mut DbJournal { conn: &conn })?;
    db::mods::update(&mut conn, |r| {
        Ok(crate::mod_engine::registry::with_all(r, false))
    })?;
    // Nothing is enabled and nothing is linked: the deployment is current.
    let (_, revision) = db::mods::load_with_revision(&conn)?;
    db::mods::set_deployed_revision(&conn, Some(revision))?;
    let text = summarise(
        ctx,
        &conn,
        EventKind::Reset,
        "Clean slate restored",
        &report,
    );
    ctx.journal(
        &conn,
        Event::new(Severity::Info, EventKind::Reset, None, text.clone()),
    );
    Ok(JobOutput::Done(text))
}

pub fn restore_overlays(ctx: &JobContext) -> Result<JobOutput> {
    let conn = ctx.db()?;
    let n = reset::restore_overlays(&ctx.steam())?;
    let text = format!("Restored {n} overlay folder(s)");
    ctx.journal(
        &conn,
        Event::new(Severity::Info, EventKind::Backup, None, text.clone()),
    );
    Ok(JobOutput::Done(text))
}

pub fn refresh_backup(ctx: &JobContext) -> Result<JobOutput> {
    let conn = ctx.db()?;
    let info = ctx.steam();
    require_game(&info)?;
    if !db::links::load(&conn)?.links.is_empty() {
        return Err(PyluneError::Invalid(
            "unlink all mods before refreshing the backup".into(),
        ));
    }
    let n = snapshot::refresh_meta_backup(&info)?;
    let text = format!("Backed up {n} metadata file(s)");
    ctx.journal(
        &conn,
        Event::new(Severity::Info, EventKind::Backup, None, text.clone()),
    );
    Ok(JobOutput::Done(text))
}
