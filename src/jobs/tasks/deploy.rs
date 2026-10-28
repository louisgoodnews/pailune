//! Exclusive jobs touching the game directory.

use super::require_game;
use crate::backup::{MetaBackup, reset, snapshot};
use crate::db;
use crate::db::links::DbJournal;
use crate::error::{PailuneError, Result};
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::logging::{Event, EventKind, Severity};
use crate::mod_engine::{model::ModEntry, plan};
use crate::overlay::{self, OverlayPlan};
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
    if snapshot::meta_backup_state(&info) == MetaBackup::Outdated {
        return Err(PailuneError::Invalid(
            "the game was updated since the metadata backup — follow the steps in the \
             yellow banner (unlink, verify in Steam, refresh backup) before applying"
                .into(),
        ));
    }
    let mut manifest = db::links::load(conn)?;
    let mut journal = DbJournal { conn };
    undeploy::undeploy(&mut manifest, &mut journal)?;
    let store = crate::inplace::Store {
        root: ctx.paths.game_backup.clone(),
    };
    crate::inplace::revert(&info.game_dir, &store)?;
    snapshot::ensure_meta_backup(&info)?;
    let (registry, revision) = db::mods::load_with_revision(conn)?;
    let mut link_plan = plan::build(&ctx.config, &ctx.paths.staging, &registry)?;
    let mut input = overlay::collect::collect(&ctx.paths.staging, &registry)?;
    let (inplace_plan, overlay) = deploy_archives(ctx, &info, &mut input)?;
    if let Some(ip) = &inplace_plan {
        link_plan.ops.extend(ip.links.clone());
    }
    link_plan
        .ops
        .extend(overlay.iter().flat_map(|o| o.links.clone()));
    let owned = [
        ctx.paths.staging.as_path(),
        ctx.paths.overlay.as_path(),
        ctx.paths.game_backup.as_path(),
    ];
    let report = deploy::deploy(&link_plan, &owned, &mut manifest, &mut journal)?;
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
    let waiting = enabled.iter().filter(|m| m.paz_files > 0).count();
    if waiting > 0 && overlay.is_none() && inplace_plan.is_none() {
        text.push_str(&format!(
            " — {waiting} mod(s) contain textures, JSON patches or unplaced files that were skipped"
        ));
    }
    if let Some(ip) = &inplace_plan {
        ip.skipped.iter().for_each(|reason| {
            ctx.journal(
                conn,
                Event::new(Severity::Warn, EventKind::Deploy, None, reason.clone()),
            )
        });
        text.push_str(&format!(
            " — in-place: {} file(s) redirected in group(s) {}{}",
            ip.file_count,
            ip.groups.join(", "),
            match ip.skipped.len() {
                0 => String::new(),
                n => format!(", {n} skipped (see Log)"),
            }
        ));
    }
    if let Some(o) = &overlay {
        o.skipped.iter().for_each(|reason| {
            ctx.journal(
                conn,
                Event::new(Severity::Warn, EventKind::Deploy, None, reason.clone()),
            )
        });
        text.push_str(&format!(
            " — archive overlay: {} file(s) packed ({} replace game files) in group(s) {}{}",
            o.packed,
            o.replaced,
            o.groups.join(", "),
            match o.skipped.len() {
                0 => String::new(),
                n => format!(", {n} skipped (see Log)"),
            }
        ));
        o.patch_notes.iter().for_each(|n| {
            let severity = if n.ok { Severity::Info } else { Severity::Warn };
            ctx.journal(
                conn,
                Event::new(
                    severity,
                    EventKind::Deploy,
                    Some(&n.mod_name),
                    n.message.clone(),
                ),
            );
        });
        let failed = o.patch_notes.iter().filter(|n| !n.ok).count();
        if !o.patch_notes.is_empty() {
            text.push_str(&format!(
                " — JSON patches: {} file(s) patched{}",
                o.patch_notes.len() - failed,
                match failed {
                    0 => String::new(),
                    n => format!(", {n} with problems (see Log)"),
                }
            ));
        }
    }
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

/// Deploys in-place archives and archive overlay groups.
fn deploy_archives(
    ctx: &JobContext,
    info: &crate::steam::SteamInfo,
    input: &mut overlay::OverlayInput,
) -> Result<(Option<crate::inplace::InPlacePlan>, Option<OverlayPlan>)> {
    if input.is_empty() && input.inplace.is_empty() {
        let _ = std::fs::remove_dir_all(&ctx.paths.overlay);
        return Ok((None, None));
    }

    use crate::error::IoContext;
    let store = crate::inplace::Store {
        root: ctx.paths.game_backup.clone(),
    };

    let patch_mods = overlay::patches::read_all(&input.patches);
    let mut wanted = overlay::patches::wanted(&patch_mods);
    wanted.paths.extend(input.packs.keys().cloned());
    wanted.paths.extend(input.inplace.keys().cloned());
    wanted.paths.extend(
        input
            .text_patches
            .iter()
            .map(|(_, target, _)| target.clone()),
    );

    let papgt_bytes = match std::fs::read(crate::symlink::bak_path(&info.meta_dir.join("0.papgt")))
    {
        Ok(b) => b,
        Err(_) => {
            let live = info.meta_dir.join("0.papgt");
            std::fs::read(&live).at(&live)?
        }
    };
    let base = crate::paz::vanilla::load_cached(&info.game_dir, &papgt_bytes, &wanted)?;

    let mut inplace_files = std::collections::BTreeMap::new();
    for (vpath, source) in std::mem::take(&mut input.inplace) {
        if base.locate(&vpath).is_some() {
            inplace_files.insert(vpath, source);
        } else {
            input.packs.insert(vpath, source);
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let (inplace_plan, current_papgt_bytes) = if !inplace_files.is_empty() {
        let plan = crate::inplace::build(
            &info.game_dir,
            &store,
            &info.meta_dir,
            &base,
            &inplace_files,
            now,
        )?;
        let serialized_papgt = crate::paz::papgt::serialize(&plan.papgt)?;
        (Some(plan), serialized_papgt)
    } else {
        (None, papgt_bytes)
    };

    let overlay_plan = if !input.is_empty() {
        let pathc = if input.packs.keys().any(|v| v.ends_with(".dds")) {
            Some(
                std::fs::read(crate::symlink::bak_path(&info.meta_dir.join("0.pathc")))
                    .or_else(|_| std::fs::read(info.meta_dir.join("0.pathc")))
                    .at(&info.meta_dir.join("0.pathc"))?,
            )
        } else {
            None
        };
        let plan = overlay::build::build_from(
            info,
            &current_papgt_bytes,
            pathc.as_deref(),
            input,
            &ctx.paths.overlay,
        )?;
        Some(plan)
    } else {
        let _ = std::fs::remove_dir_all(&ctx.paths.overlay);
        None
    };

    Ok((inplace_plan, overlay_plan))
}

/// 1-based load-order position of a mod.
fn position(mods: &[ModEntry], name: &str) -> usize {
    mods.iter()
        .position(|m| m.name == name)
        .map_or(0, |i| i + 1)
}

/// Deploys the enabled mods (see [`redeploy`]).
pub fn apply(ctx: &JobContext) -> Result<JobOutput> {
    let mut conn = ctx.db()?;
    redeploy(ctx, &mut conn).map(JobOutput::Done)
}

/// Removes every deployed link and restores shadowed originals.
pub fn unlink_all(ctx: &JobContext) -> Result<JobOutput> {
    let conn = ctx.db()?;
    require_game(&ctx.steam())?;
    let mut manifest = db::links::load(&conn)?;
    let report = undeploy::undeploy(&mut manifest, &mut DbJournal { conn: &conn })?;
    db::mods::set_deployed_revision(&conn, None)?;
    let store = crate::inplace::Store {
        root: ctx.paths.game_backup.clone(),
    };
    crate::inplace::revert(&ctx.steam().game_dir, &store)?;
    let text = summarise(ctx, &conn, EventKind::Unlink, "Unlinked", &report);
    ctx.journal(
        &conn,
        Event::new(Severity::Info, EventKind::Unlink, None, text.clone()),
    );
    Ok(JobOutput::Done(text))
}

/// Returns the game to vanilla and disables every mod.
pub fn clean_slate(ctx: &JobContext) -> Result<JobOutput> {
    let mut conn = ctx.db()?;
    let info = ctx.steam();
    require_game(&info)?;
    let mut manifest = db::links::load(&conn)?;
    let report = reset::clean_slate(&info, &mut manifest, &mut DbJournal { conn: &conn })?;
    let store = crate::inplace::Store {
        root: ctx.paths.game_backup.clone(),
    };
    crate::inplace::revert(&info.game_dir, &store)?;
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

/// Moves archive folders set aside by Clean Slate back into place.
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

/// Re-snapshots `meta/` to `.bak` (only while nothing is deployed).
pub fn refresh_backup(ctx: &JobContext) -> Result<JobOutput> {
    let conn = ctx.db()?;
    let info = ctx.steam();
    require_game(&info)?;
    if !db::links::load(&conn)?.links.is_empty() {
        return Err(PailuneError::Invalid(
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
