//! Resolving download links and streaming archives to disk.

use super::api::Client;
use super::{DownloadLink, NexusFile, NexusMod, NxmLink};
use crate::error::{IoContext, PyluneError, Result};
use crate::mod_engine::install;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// What to download and how to stage it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadJob {
    pub mod_id: u64,
    pub file_id: u64,
    /// Present when started from an `nxm://` link (needed for free accounts).
    pub nxm: Option<NxmLink>,
    /// Existing staged mod to replace (update), if any.
    pub replace: Option<String>,
}

/// A downloaded archive with the metadata needed to stage it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub archive: PathBuf,
    pub info: NexusMod,
    pub file: NexusFile,
}

/// Progress callback: `(bytes done, total bytes)`.
pub type Progress<'a> = &'a dyn Fn(u64, Option<u64>);

/// A reader that reports how many bytes passed through it.
struct Counting<'a, R> {
    inner: R,
    done: u64,
    reported: u64,
    total: Option<u64>,
    progress: Progress<'a>,
}

impl<R: Read> Read for Counting<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.done += n as u64;
        // Throttle UI updates to roughly every 256 KiB.
        if self.done - self.reported >= 256 * 1024 || n == 0 {
            self.reported = self.done;
            (self.progress)(self.done, self.total);
        }
        Ok(n)
    }
}

/// Streams `uri` into `dest` via a `.part` file that is renamed on success
/// and removed on failure.
fn stream(client: &Client, uri: &str, dest: &Path, progress: Progress) -> Result<()> {
    let part = dest.with_extension("part");
    let copied = client.open_download(uri).and_then(|(total, reader)| {
        let mut file = fs::File::create(&part).at(&part)?;
        let mut counting = Counting {
            inner: reader,
            done: 0,
            reported: 0,
            total,
            progress,
        };
        io::copy(&mut counting, &mut file).at(&part).map(drop)
    });
    match copied {
        Ok(()) => fs::rename(&part, dest).at(dest),
        Err(err) => {
            let _ = fs::remove_file(&part);
            Err(err)
        }
    }
}

/// Tries each mirror in turn until one succeeds; the last failure is kept.
fn download_any(
    client: &Client,
    mirrors: &[DownloadLink],
    archive: &Path,
    progress: Progress,
) -> Result<()> {
    let none = Err(PyluneError::Invalid(
        "Nexus returned no download mirrors".into(),
    ));
    mirrors.iter().fold(none, |result, link| match result {
        Ok(()) => Ok(()),
        Err(_) => stream(client, &link.uri, archive, progress).inspect_err(|err| {
            tracing::warn!(mirror = %link.short_name, %err, "download mirror failed");
        }),
    })
}

/// Fails early with a clear message when an `nxm://` link has expired.
fn check_expiry(job: &DownloadJob) -> Result<()> {
    let now = crate::db::now() as u64;
    match job.nxm.as_ref().and_then(|link| link.expires) {
        Some(expires) if expires < now => Err(PyluneError::Invalid(
            "this nxm:// link has expired — click \"Mod Manager Download\" on the website again"
                .into(),
        )),
        _ => Ok(()),
    }
}

/// Looks up the file, resolves a mirror and downloads it into `downloads`.
pub fn fetch(
    client: &Client,
    job: &DownloadJob,
    downloads: &Path,
    progress: Progress,
) -> Result<Fetched> {
    check_expiry(job)?;
    let info = client.mod_info(job.mod_id)?;
    let file = client
        .files(job.mod_id)?
        .files
        .into_iter()
        .find(|f| f.file_id == job.file_id)
        .ok_or_else(|| {
            PyluneError::Invalid(format!(
                "file {} not found on mod {}",
                job.file_id, job.mod_id
            ))
        })?;
    let mirrors = client.download_links(job.mod_id, job.file_id, job.nxm.as_ref())?;
    fs::create_dir_all(downloads).at(downloads)?;
    // Unique per job: two downloads of the same file never share a path.
    let archive = downloads.join(format!(
        "{}-{}-{}",
        job.file_id,
        install::unique_suffix(),
        install::sanitize_name(&file.file_name)
    ));
    download_any(client, &mirrors, &archive, progress)?;
    Ok(Fetched {
        archive,
        info,
        file,
    })
}
