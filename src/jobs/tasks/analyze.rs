//! Conflict analysis job.

use crate::db;
use crate::error::Result;
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::mod_engine::conflicts;

/// Analyses file conflicts between the enabled mods; `generation` tags
/// the result so the UI can drop stale ones.
pub fn conflicts(ctx: &JobContext, generation: u64) -> Result<JobOutput> {
    let registry = db::mods::load(&ctx.db()?)?;
    let mut report = conflicts::analyze(&ctx.config, &ctx.paths.staging, &registry)?;
    // JSON patches conflict only where their changes overlap; that needs
    // the game's data, so a game that cannot be read just skips it.
    let info = crate::steam::locate::resolve(&ctx.config);
    let overlaps = crate::overlay::collect::collect(&ctx.paths.staging, &registry)
        .and_then(|input| crate::overlay::value_conflicts::overlaps(&info, &input.patches));
    match overlaps {
        Ok(found) => conflicts::add_overlaps(
            &mut report,
            found
                .into_iter()
                .map(|o| (o.vpath, o.earlier, o.later, o.values.into_iter().collect())),
        ),
        Err(err) => tracing::warn!(%err, "JSON patch conflicts not analysed"),
    }
    Ok(JobOutput::Conflicts(generation, report))
}
