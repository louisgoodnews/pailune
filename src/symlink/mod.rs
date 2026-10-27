//! Symlink deployment with `.bak` preservation of shadowed originals.
//!
//! Every link Pailune creates is recorded in a [`Manifest`], so a revert
//! removes exactly what was added and nothing else.

pub mod deploy;
pub mod journal;
pub mod undeploy;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Suffix appended to originals that a mod shadows.
pub const BAK_SUFFIX: &str = ".bak";

/// One deployed symlink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkRecord {
    /// Symlink location inside the game directory.
    pub target: PathBuf,
    /// Staged file the symlink points to.
    pub source: PathBuf,
    /// Where the shadowed original was moved, if there was one.
    pub backup: Option<PathBuf>,
    /// For hard links and copies (no symlinks): the placed file's stamp,
    /// proving later that it is still ours (decision D-11).
    #[serde(default)]
    pub stamp: Option<FileStamp>,
}

/// Size and modification time of a placed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStamp {
    pub size: u64,
    /// Nanoseconds since the Unix epoch.
    pub modified_ns: i64,
}

/// The stamp of a regular file (not following symlinks).
pub fn stamp(path: &Path) -> Option<FileStamp> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let ns = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(FileStamp {
        size: meta.len(),
        modified_ns: i64::try_from(ns).ok()?,
    })
}

/// Byte-for-byte comparison of two regular files.
pub fn same_contents(a: &Path, b: &Path) -> bool {
    use std::io::Read;
    let size = |p: &Path| std::fs::metadata(p).map(|m| m.len()).ok();
    let open = |p: &Path| std::fs::File::open(p).map(std::io::BufReader::new);
    let (Ok(mut fa), Ok(mut fb)) = (open(a), open(b)) else {
        return false;
    };
    if size(a).is_none() || size(a) != size(b) {
        return false;
    }
    // Compared in chunks: game files can be gigabytes.
    let (mut ba, mut bb) = (vec![0u8; 1 << 16], vec![0u8; 1 << 16]);
    loop {
        let n = match fa.read(&mut ba) {
            Ok(n) => n,
            Err(_) => return false,
        };
        if n == 0 {
            return fb.read(&mut bb[..1]).is_ok_and(|m| m == 0);
        }
        if fb.read_exact(&mut bb[..n]).is_err() || ba[..n] != bb[..n] {
            return false;
        }
    }
}

/// Everything Pailune currently has deployed into the game directory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Links in creation order.
    pub links: Vec<LinkRecord>,
    /// Directories created to host links, in creation order.
    pub created_dirs: Vec<PathBuf>,
}

/// Summary of a deploy or undeploy pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Links created or removed.
    pub linked: usize,
    /// Originals moved to or restored from `.bak`.
    pub backed_up: usize,
    /// Human-readable problems that did not abort the pass.
    pub warnings: Vec<String>,
}

/// What currently occupies a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathState {
    /// Nothing is there.
    Missing,
    /// A symlink pointing at the contained path.
    Symlink(PathBuf),
    /// A regular file.
    File,
    /// A real directory.
    Dir,
}

/// Inspects `path` without following symlinks.
pub fn path_state(path: &Path) -> PathState {
    match std::fs::symlink_metadata(path) {
        Err(_) => PathState::Missing,
        Ok(meta) if meta.file_type().is_symlink() => {
            PathState::Symlink(std::fs::read_link(path).unwrap_or_default())
        }
        Ok(meta) if meta.is_dir() => PathState::Dir,
        Ok(_) => PathState::File,
    }
}

/// `path` with [`BAK_SUFFIX`] appended to its file name.
pub fn bak_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(BAK_SUFFIX);
    path.with_file_name(name)
}
