//! Download pipeline: resolve links, stream the archive, stage it as a mod.

use super::api::Client;
use super::{NexusRef, NxmLink};
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

/// A finished download, staged under `name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub name: String,
    pub reference: NexusRef,
}

/// Progress callback: `(file name, bytes done, total bytes)`.
pub type Progress<'a> = &'a dyn Fn(&str, u64, Option<u64>);

/// A reader that reports how many bytes passed through it.
struct Counting<'a, R> {
    inner: R,
    done: u64,
    reported: u64,
    total: Option<u64>,
    label: &'a str,
    progress: Progress<'a>,
}

impl<R: Read> Read for Counting<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.done += n as u64;
        // Throttle UI updates to roughly every 256 KiB.
        if self.done - self.reported >= 256 * 1024 || n == 0 {
            self.reported = self.done;
            (self.progress)(self.label, self.done, self.total);
        }
        Ok(n)
    }
}

/// Streams `uri` into `dest`, via a `.part` file renamed on success.
fn fetch(client: &Client, uri: &str, dest: &Path, progress: Progress) -> Result<()> {
    let label = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (total, reader) = client.open_download(uri)?;
    let part = dest.with_extension("part");
    let mut file = fs::File::create(&part).at(&part)?;
    let mut counting = Counting {
        inner: reader,
        done: 0,
        reported: 0,
        total,
        label: &label,
        progress,
    };
    io::copy(&mut counting, &mut file).at(&part)?;
    fs::rename(&part, dest).at(dest)
}

/// Picks a staging name that does not clash with unrelated mods.
fn staging_name(job: &DownloadJob, mod_name: &str, staging: &Path) -> (String, bool) {
    match &job.replace {
        Some(existing) => (existing.clone(), true),
        None => {
            let base = install::sanitize_name(mod_name);
            match staging.join(&base).exists() {
                false => (base, false),
                true => (format!("{base} ({})", job.file_id), false),
            }
        }
    }
}

/// Downloads a Nexus file and stages it as a mod.
pub fn run(
    client: &Client,
    job: &DownloadJob,
    staging: &Path,
    downloads: &Path,
    progress: Progress,
) -> Result<Installed> {
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
    let link = client
        .download_links(job.mod_id, job.file_id, job.nxm.as_ref())?
        .into_iter()
        .next()
        .ok_or_else(|| PyluneError::Invalid("Nexus returned no download mirrors".into()))?;
    fs::create_dir_all(downloads).at(downloads)?;
    let archive: PathBuf = downloads.join(install::sanitize_name(&file.file_name));
    fetch(client, &link.uri, &archive, progress)?;
    let (name, replace) = staging_name(job, &info.name, staging);
    let staged = install::import_as(&archive, staging, &name, replace);
    let _ = fs::remove_file(&archive);
    Ok(Installed {
        name: staged?,
        reference: NexusRef {
            mod_id: job.mod_id,
            file_id: job.file_id,
            version: info.version,
        },
    })
}
