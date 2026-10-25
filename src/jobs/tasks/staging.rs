//! Installing, updating and deleting staged mods.

use super::deploy::redeploy;
use crate::db;
use crate::error::{IoContext, PyluneError, Result};
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::logging::{Event, EventKind, Severity, journal};
use crate::mod_engine::model::NexusRef;
use crate::mod_engine::{info, install, registry};
use std::fs;
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
    let details = info::details(&ctx.paths.staging.join(name))?;
    let lines: Vec<String> = details
        .files
        .iter()
        .map(|f| {
            format!(
                "{} ({} bytes{})",
                f.rel.display(),
                f.bytes,
                if f.deployable { "" } else { ", not deployed" }
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

pub fn remove(ctx: &JobContext, name: &str) -> Result<JobOutput> {
    let mut conn = ctx.db()?;
    if db::mods::load(&conn)?.mods.iter().all(|m| m.name != name) {
        return Err(PyluneError::Invalid(format!("no mod named '{name}'")));
    }
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
    fs::remove_dir_all(&mod_dir).at(&mod_dir)?;
    ctx.journal(
        &conn,
        Event::new(
            Severity::Info,
            EventKind::Remove,
            Some(name),
            "deleted from staging",
        ),
    );
    Ok(JobOutput::Done(match redeployed {
        Some(text) => format!("Deleted '{name}'; {text}"),
        None => format!("Deleted '{name}'"),
    }))
}
