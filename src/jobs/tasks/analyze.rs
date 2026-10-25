//! Conflict analysis job.

use crate::db;
use crate::error::Result;
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::mod_engine::conflicts;

pub fn conflicts(ctx: &JobContext, generation: u64) -> Result<JobOutput> {
    let registry = db::mods::load(&ctx.db()?)?;
    let report = conflicts::analyze(&ctx.config, &ctx.paths.staging, &registry)?;
    Ok(JobOutput::Conflicts(generation, report))
}
