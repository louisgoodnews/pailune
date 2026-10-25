//! Nexus Mods jobs: account, lookups, downloads and update checks.

use super::staging::register;
use crate::db;
use crate::error::{PyluneError, Result};
use crate::jobs::JobOutput;
use crate::jobs::context::JobContext;
use crate::logging::{Event, EventKind, Severity};
use crate::mod_engine::install;
use crate::mod_engine::model::NexusRef;
use crate::nexus::api::Client;
use crate::nexus::download::{self, DownloadJob};
use crate::nexus::key;
use std::fs;
use std::thread;

/// Parallel requests used by the update check.
const UPDATE_CHECK_PARALLELISM: usize = 4;

fn client(ctx: &JobContext) -> Result<Client> {
    key::load(&ctx.paths.nexus_key)
        .map(|k| Client::new(&k, &ctx.config.nexus.domain))
        .ok_or_else(|| PyluneError::Invalid("set a Nexus API key first".into()))
}

pub fn validate(ctx: &JobContext) -> Result<JobOutput> {
    client(ctx)?.validate().map(JobOutput::User)
}

pub fn fetch_mod(ctx: &JobContext, mod_id: u64) -> Result<JobOutput> {
    let client = client(ctx)?;
    let info = client.mod_info(mod_id)?;
    Ok(JobOutput::Mod(info, client.files(mod_id)?.files))
}

/// Picks a staging name that does not clash with unrelated mods.
fn staging_name(ctx: &JobContext, job: &DownloadJob, mod_name: &str) -> (String, bool) {
    match &job.replace {
        Some(existing) => (existing.clone(), true),
        None => {
            let base = install::sanitize_name(mod_name);
            match ctx.paths.staging.join(&base).exists() {
                false => (base, false),
                true => (format!("{base} ({})", job.file_id), false),
            }
        }
    }
}

pub fn download(ctx: &JobContext, job: &DownloadJob) -> Result<JobOutput> {
    let client = client(ctx)?;
    let fetched = download::fetch(&client, job, &ctx.paths.downloads, &|done, total| {
        ctx.progress(done, total)
    })?;
    let (name, replace) = staging_name(ctx, job, &fetched.info.name);
    let staged = match replace {
        true => {
            let _guard = ctx.lock_game();
            install::import_as(&fetched.archive, &ctx.paths.staging, &name, true)
        }
        false => install::import_as(&fetched.archive, &ctx.paths.staging, &name, false),
    };
    let _ = fs::remove_file(&fetched.archive);
    let name = staged?;
    let reference = NexusRef {
        mod_id: job.mod_id,
        file_id: job.file_id,
        version: fetched.info.version.clone(),
    };
    let source = format!(
        "nexus:{}/mods/{}/files/{} ({})",
        ctx.config.nexus.domain, job.mod_id, job.file_id, fetched.file.file_name
    );
    register(ctx, &name, &source, Some(&reference), replace).map(JobOutput::Done)
}

/// Compares every Nexus-installed mod with its current version, querying
/// several mods concurrently.
pub fn check_updates(ctx: &JobContext) -> Result<JobOutput> {
    let client = client(ctx)?;
    let conn = ctx.db()?;
    let refs: Vec<(String, NexusRef)> = db::mods::load(&conn)?
        .mods
        .into_iter()
        .filter_map(|m| m.nexus.map(|n| (m.name, n)))
        .collect();
    let total = refs.len() as u64;
    let latest: Vec<(String, String, String)> = refs
        .chunks(UPDATE_CHECK_PARALLELISM)
        .enumerate()
        .map(|(chunk_index, chunk)| {
            ctx.progress((chunk_index * UPDATE_CHECK_PARALLELISM) as u64, Some(total));
            thread::scope(|scope| {
                chunk
                    .iter()
                    .map(|(name, r)| {
                        let client = &client;
                        scope.spawn(move || {
                            client
                                .mod_info(r.mod_id)
                                .map(|info| (name.clone(), r.version.clone(), info.version))
                        })
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|handle| {
                        handle.join().unwrap_or_else(|_| {
                            Err(PyluneError::Invalid("update check crashed".into()))
                        })
                    })
                    .collect::<Result<Vec<_>>>()
            })
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    let outdated: Vec<(String, String)> = latest
        .into_iter()
        .filter(|(_, have, now)| !now.is_empty() && have != now)
        .map(|(name, _, now)| (name, now))
        .collect();
    outdated.iter().for_each(|(name, version)| {
        let event = Event::new(
            Severity::Info,
            EventKind::Update,
            Some(name),
            format!("version {version} available"),
        );
        ctx.journal(&conn, event);
    });
    Ok(JobOutput::Updates(outdated))
}
