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
use crate::nexus::{FileSummary, key, updates};
use std::fs;
use std::thread;

/// Parallel requests used by the update check.
const UPDATE_CHECK_PARALLELISM: usize = 4;

/// A client for the configured game and API root.
fn nexus_client(ctx: &JobContext, key: &str) -> Client {
    let api = ctx
        .config
        .nexus
        .api_url
        .as_deref()
        .unwrap_or(crate::nexus::api::DEFAULT_API);
    Client::with_api(key, &ctx.config.nexus.domain, api)
}

/// A client authenticated with the stored API key.
fn client(ctx: &JobContext) -> Result<Client> {
    key::load(&ctx.paths.nexus_key)
        .map(|k| nexus_client(ctx, &k))
        .ok_or_else(|| PyluneError::Invalid("set a Nexus API key first".into()))
}

/// Checks the stored API key and returns the account.
pub fn validate(ctx: &JobContext) -> Result<JobOutput> {
    client(ctx)?.validate().map(JobOutput::User)
}

/// Fetches a mod page and its files.
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

/// Downloads a Nexus file and stages it (replacing an existing mod for
/// updates), recording its origin and collection membership.
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
    let message = register(ctx, &name, &source, Some(&reference), replace)?;
    if let Some(slug) = &job.collection {
        let conn = ctx.db()?;
        db::collections::link_member(&conn, slug, job.mod_id, job.file_id, &name)?;
        let event = Event::new(
            Severity::Info,
            EventKind::Install,
            Some(&name),
            format!("member of collection {slug}"),
        );
        ctx.journal(&conn, event);
    }
    Ok(JobOutput::Done(message))
}

/// Result of checking one mod: `Some(label)` when a newer file exists.
fn check_one(client: &Client, origin: &NexusRef) -> Result<Option<String>> {
    let list = client.files(origin.mod_id)?;
    Ok(
        updates::latest_successor(&list.file_updates, origin.file_id)
            .map(|id| updates::describe(&list, id)),
    )
}

/// Checks a few mods concurrently, one thread each.
fn check_chunk(
    client: &Client,
    chunk: &[(String, NexusRef)],
) -> Vec<(String, Result<Option<String>>)> {
    thread::scope(|scope| {
        let handles: Vec<_> = chunk
            .iter()
            .map(|(name, origin)| (name.clone(), scope.spawn(|| check_one(client, origin))))
            .collect();
        let crashed = || Err(PyluneError::Invalid("update check crashed".into()));
        handles
            .into_iter()
            .map(|(name, handle)| (name, handle.join().unwrap_or_else(|_| crashed())))
            .collect()
    })
}

/// Follows each Nexus-installed file's update chain, several mods at a
/// time. A failing mod (hidden, deleted, rate limit) is reported on its own
/// instead of aborting the whole check.
pub fn check_updates(ctx: &JobContext) -> Result<JobOutput> {
    let client = client(ctx)?;
    let conn = ctx.db()?;
    let refs: Vec<(String, NexusRef)> = db::mods::load(&conn)?
        .mods
        .into_iter()
        .filter_map(|m| m.nexus.map(|n| (m.name, n)))
        .collect();
    let total = refs.len() as u64;
    let results: Vec<(String, Result<Option<String>>)> = refs
        .chunks(UPDATE_CHECK_PARALLELISM)
        .enumerate()
        .flat_map(|(chunk_index, chunk)| {
            ctx.progress((chunk_index * UPDATE_CHECK_PARALLELISM) as u64, Some(total));
            check_chunk(&client, chunk)
        })
        .collect();
    let journal = |name: &str, severity, message: String| {
        ctx.journal(
            &conn,
            Event::new(severity, EventKind::Update, Some(name), message),
        );
    };
    let (outdated, failed) =
        results
            .into_iter()
            .fold(
                (Vec::new(), 0),
                |(mut outdated, failed), (name, result)| match result {
                    Ok(Some(label)) => {
                        journal(&name, Severity::Info, format!("update available: {label}"));
                        outdated.push((name, label));
                        (outdated, failed)
                    }
                    Ok(None) => (outdated, failed),
                    Err(err) => {
                        journal(&name, Severity::Warn, format!("update check failed: {err}"));
                        (outdated, failed + 1)
                    }
                },
            );
    Ok(JobOutput::Updates { outdated, failed })
}

/// Names and size of one file, shown before an `nxm://` download starts.
pub fn describe_file(ctx: &JobContext, mod_id: u64, file_id: u64) -> Result<JobOutput> {
    let client = client(ctx)?;
    let info = client.mod_info(mod_id)?;
    let file = client
        .files(mod_id)?
        .files
        .into_iter()
        .find(|f| f.file_id == file_id);
    Ok(JobOutput::Described(FileSummary {
        mod_id,
        file_id,
        mod_name: info.name,
        file_name: file
            .as_ref()
            .map_or_else(|| format!("file {file_id}"), |f| f.name.clone()),
        version: file.as_ref().map(|f| f.version.clone()).unwrap_or_default(),
        size_kb: file.map_or(0, |f| f.size_kb),
    }))
}

/// Fetches a collection revision from the v2 GraphQL API.
pub fn fetch_collection(ctx: &JobContext, slug: &str, revision: Option<u32>) -> Result<JobOutput> {
    // Collection listings are public; a key is sent when one is stored.
    let key = key::load(&ctx.paths.nexus_key).unwrap_or_default();
    nexus_client(ctx, &key)
        .collection(slug, revision)
        .map(JobOutput::Collection)
}
