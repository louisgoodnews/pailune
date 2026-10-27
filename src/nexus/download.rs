//! Resolving download links and streaming archives to disk.

use super::api::Client;
use super::{DownloadLink, NexusFile, NexusMod, NxmLink};
use crate::error::{IoContext, PailuneError, Result};
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
    /// Slug of the collection this file is installed for, if any.
    pub collection: Option<String>,
}

/// A downloaded archive with the metadata needed to stage it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub archive: PathBuf,
    pub info: NexusMod,
    pub file: NexusFile,
    pub integrity: Integrity,
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

/// Attempts per mirror; later attempts resume where the last one stopped.
const ATTEMPTS: usize = 4;

/// Downloads into `part`, resuming from its current length when the
/// server supports ranges (otherwise the file restarts).
fn attempt(client: &Client, uri: &str, part: &Path, progress: Progress, left: usize) -> Result<()> {
    let have = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    let copied = client
        .open_download(uri, have)
        .and_then(|(remaining, partial, reader)| {
            let start = if partial { have } else { 0 };
            let mut file = match partial {
                true => fs::OpenOptions::new().append(true).open(part).at(part)?,
                false => fs::File::create(part).at(part)?,
            };
            let total = remaining.map(|r| r + start);
            let mut counting = Counting {
                inner: reader,
                done: start,
                reported: start,
                total,
                progress,
            };
            io::copy(&mut counting, &mut file).at(part).map(drop)
        });
    match (copied, left) {
        (Ok(()), _) => Ok(()),
        (Err(err), 0) => Err(err),
        (Err(err), _) => {
            tracing::warn!(%err, "download interrupted, resuming");
            attempt(client, uri, part, progress, left - 1)
        }
    }
}

/// Streams `uri` into `dest` via a `.part` file that is renamed on success
/// and removed on failure.
fn stream(client: &Client, uri: &str, dest: &Path, progress: Progress) -> Result<()> {
    let part = dest.with_extension("part");
    let _ = fs::remove_file(&part);
    match attempt(client, uri, &part, progress, ATTEMPTS - 1) {
        Ok(()) => fs::rename(&part, dest).at(dest),
        Err(err) => {
            let _ = fs::remove_file(&part);
            Err(err)
        }
    }
}

/// How a download's content was checked against Nexus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integrity {
    /// The MD5 is registered for exactly this file.
    Verified,
    /// Nexus does not know the MD5 (yet); size matched.
    Unverified,
}

/// Feeds written bytes into an MD5 hasher.
struct Md5Writer(md5::Md5);

impl io::Write for Md5Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        md5::Digest::update(&mut self.0, buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// MD5 of a file as lowercase hex.
fn md5_hex(path: &Path) -> Result<String> {
    use md5::Digest;
    let mut writer = Md5Writer(md5::Md5::new());
    io::copy(&mut fs::File::open(path).at(path)?, &mut writer).at(path)?;
    Ok(writer
        .0
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Asks Nexus which file has this content; a hit for another file fails.
fn verify(client: &Client, archive: &Path, job: &DownloadJob) -> Result<Integrity> {
    let hits = client.md5_search(&md5_hex(archive)?)?;
    match (hits.is_empty(), hits.contains(&(job.mod_id, job.file_id))) {
        (true, _) => Ok(Integrity::Unverified),
        (false, true) => Ok(Integrity::Verified),
        (false, false) => Err(PailuneError::Invalid(format!(
            "downloaded content belongs to a different Nexus file than {}/{}",
            job.mod_id, job.file_id
        ))),
    }
}

/// Tries each mirror in turn until one succeeds; the last failure is kept.
fn download_any(
    client: &Client,
    mirrors: &[DownloadLink],
    archive: &Path,
    progress: Progress,
) -> Result<()> {
    let none = Err(PailuneError::Invalid(
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
        Some(expires) if expires < now => Err(PailuneError::Invalid(
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
            PailuneError::Invalid(format!(
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
    // A truncated or substituted file is caught before it is staged.
    let size = fs::metadata(&archive).at(&archive)?.len();
    if let Some(expected) = file.size_in_bytes.filter(|e| *e != size) {
        let _ = fs::remove_file(&archive);
        return Err(PailuneError::Invalid(format!(
            "{}: downloaded {size} bytes, Nexus lists {expected}",
            file.file_name
        )));
    }
    let integrity = verify(client, &archive, job).inspect_err(|_| {
        let _ = fs::remove_file(&archive);
    })?;
    Ok(Fetched {
        archive,
        info,
        file,
        integrity,
    })
}
