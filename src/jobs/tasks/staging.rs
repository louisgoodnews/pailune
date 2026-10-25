//! Installing, updating and deleting staged mods.

use super::deploy::redeploy;
use crate::db;
use crate::error::{PyluneError, Result};
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::logging::{Event, EventKind, Severity, journal};
use crate::mod_engine::model::NexusRef;
use crate::mod_engine::{info, install, registry, trash};
use std::path::Path;

/// Registers a freshly staged mod and writes its install log.
pub(super) fn register(
    ctx: &JobContext,
    name: &str,
    source: &str,
    nexus: Option<&NexusRef>,
    updated: bool,
) -> Result<String> {
    let mut conn = ctx.db()?;
    let now = db::now();
    // Walk the files before taking the write lock (A-20).
    let scan = registry::scan(&ctx.paths.staging)?;
    db::mods::update(&mut conn, |r| Ok(registry::merge(r, &scan, now)))?;
    db::mods::record_install(&conn, name, source, nexus, now)?;
    let details = info::details(&ctx.paths.staging.join(name), None)?;
    let lines: Vec<String> = details
        .files
        .iter()
        .map(|f| {
            format!(
                "{} ({} bytes, {})",
                f.rel.display(),
                f.bytes,
                f.placement.note()
            )
        })
        .collect();
    let (kind, verb) = if updated {
        (EventKind::Update, "updated")
    } else {
        (EventKind::Install, "installed")
    };
    let message = format!(
        "{verb} from {source}: {} file(s), {} bytes",
        details.files.len(),
        details.total_bytes
    );
    ctx.journal(&conn, Event::new(Severity::Info, kind, Some(name), message));
    journal::mod_detail(&ctx.paths.mod_logs, name, "  files:", &lines)?;
    let enabled = db::mods::load(&conn)?
        .mods
        .iter()
        .any(|m| m.name == name && m.enabled);
    Ok(match (updated, enabled) {
        (true, true) => format!("Updated '{name}' — press Apply to redeploy"),
        (true, false) => format!("Updated '{name}'"),
        (false, _) => format!("Installed '{name}' — enable it and press Apply"),
    })
}

/// Stages a file, archive or folder; `replace` updates an existing mod.
pub fn import(ctx: &JobContext, source: &Path, replace: Option<String>) -> Result<JobOutput> {
    let name = match &replace {
        // Replacing a possibly deployed mod must not race a deployment.
        Some(existing) => {
            let _guard = ctx.lock_game();
            install::import_as(source, &ctx.paths.staging, existing, true)?
        }
        None => install::import(source, &ctx.paths.staging)?,
    };
    let source_text = source.display().to_string();
    register(ctx, &name, &source_text, None, replace.is_some()).map(JobOutput::Done)
}

/// Deletes a staged mod, redeploying first if links point into it.
pub fn remove(ctx: &JobContext, name: &str) -> Result<JobOutput> {
    let mut conn = ctx.db()?;
    let entry = db::mods::load(&conn)?
        .mods
        .into_iter()
        .find(|m| m.name == name)
        .ok_or_else(|| PyluneError::Invalid(format!("no mod named '{name}'")))?;
    // Decide by what is actually linked, not by the (possibly changed) flag.
    let mod_dir = ctx.paths.staging.join(name);
    let deployed = db::links::load(&conn)?
        .links
        .iter()
        .any(|l| l.source.starts_with(&mod_dir));
    if deployed {
        super::require_game(&ctx.steam())?;
    }
    // Drop the mod from the registry first so a redeploy no longer links it.
    db::mods::update(&mut conn, |r| Ok(registry::without(r, name)))?;
    let redeployed = match deployed {
        true => Some(redeploy(ctx, &mut conn)?),
        false => None,
    };
    // Recoverable: the folder moves to the trash with its metadata.
    trash::move_to_trash(&ctx.paths.staging, &ctx.paths.trash, &entry, db::now())?;
    trash::prune(&ctx.paths.trash, trash::KEEP)?;
    ctx.journal(
        &conn,
        Event::new(
            Severity::Info,
            EventKind::Remove,
            Some(name),
            "moved to the trash",
        ),
    );
    crate::logging::journal::retire_mod_log(&ctx.paths.mod_logs, name);
    Ok(JobOutput::Done(match redeployed {
        Some(text) => format!("Deleted '{name}' (restorable); {text}"),
        None => format!("Deleted '{name}' (restorable)"),
    }))
}

/// Brings a deleted mod back from the trash with its metadata (origin,
/// variant); it returns disabled at the end of the load order.
pub fn restore_deleted(ctx: &JobContext, id: &str) -> Result<JobOutput> {
    let (name, item) = trash::restore(&ctx.paths.trash, &ctx.paths.staging, id)?;
    let mut conn = ctx.db()?;
    let scan = registry::scan(&ctx.paths.staging)?;
    let now = db::now();
    db::mods::update(&mut conn, |r| {
        Ok(registry::with_variant(
            &registry::merge(r, &scan, now),
            &name,
            item.variant.clone(),
        ))
    })?;
    db::mods::record_install(&conn, &name, &item.source, item.nexus.as_ref(), now)?;
    ctx.journal(
        &conn,
        Event::new(
            Severity::Info,
            EventKind::Install,
            Some(&name),
            "restored from the trash",
        ),
    );
    Ok(JobOutput::Done(format!(
        "Restored '{name}' — enable it and press Apply"
    )))
}

/// Deletes a trashed mod for good.
pub fn purge_deleted(ctx: &JobContext, id: &str) -> Result<JobOutput> {
    trash::purge(&ctx.paths.trash, id)?;
    Ok(JobOutput::Done("Deleted permanently".into()))
}
