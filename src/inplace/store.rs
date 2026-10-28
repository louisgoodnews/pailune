//! The backup store of the in-place route (D-14): one **base** (the vanilla
//! index files of the game build) and numbered **versions** (the index
//! files Pailune wrote at each Apply). Restoring either puts exactly those
//! bytes back; hashes in every manifest prove what was stored.

use crate::error::{IoContext, PailuneError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// One stored file: its path relative to the game folder and its hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stored {
    pub rel: String,
    pub sha256: String,
}

/// A large game archive that must stay untouched: size and hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub rel: String,
    pub size: u64,
    pub sha256: String,
}

/// What a base or version folder holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// 0 for the base, 1… for versions.
    pub number: u32,
    pub created_at: i64,
    pub label: String,
    /// `meta/0.paver` of the build this belongs to, hex.
    pub game_build: String,
    pub files: Vec<Stored>,
    /// Base only: the vanilla archives of the covered groups.
    #[serde(default)]
    pub archives: Vec<Fingerprint>,
    /// Versions only: Pailune's added archives, game-relative path → content
    /// hash in the store's `archives/` folder (linked into the game).
    #[serde(default)]
    pub links: Vec<Stored>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of bytes.
pub fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// SHA-256 of a (possibly huge) file, streamed.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).at(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).at(path)?;
        if n == 0 {
            return Ok(hex(&hasher.finalize()));
        }
        hasher.update(&buf[..n]);
    }
}

/// Writes `bytes` to `path` through a temporary file and a rename.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).at(dir)?;
    }
    let temp = path.with_extension("pailune-tmp");
    fs::write(&temp, bytes).at(&temp)?;
    fs::rename(&temp, path).at(path)
}

/// What to store: the files (`rel` → bytes) and what belongs to them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub label: String,
    pub game_build: String,
    pub files: Vec<(String, Vec<u8>)>,
    /// Base only.
    pub archives: Vec<Fingerprint>,
    /// Versions only.
    pub links: Vec<Stored>,
}

/// The store under `root` (`<data>/game-backup`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn folder(&self, number: u32) -> PathBuf {
        match number {
            0 => self.root.join("base"),
            n => self.root.join("versions").join(n.to_string()),
        }
    }

    /// Whether the base backup (0) exists.
    pub fn has_base(&self) -> bool {
        self.folder(0).join("manifest.json").exists()
    }

    /// The manifest of the base (0) or a version.
    pub fn manifest(&self, number: u32) -> Result<Manifest> {
        let path = self.folder(number).join("manifest.json");
        Ok(serde_json::from_slice(&fs::read(&path).at(&path)?)?)
    }

    /// Stored versions, oldest first (without the base).
    pub fn versions(&self) -> Vec<Manifest> {
        let mut found: Vec<Manifest> = fs::read_dir(self.root.join("versions"))
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok()?.file_name().to_str()?.parse().ok())
            .filter_map(|n| self.manifest(n).ok())
            .collect();
        found.sort_by_key(|m| m.number);
        found
    }

    /// Stores a snapshot as the base (0) or the next version.
    pub fn save(&self, base: bool, snapshot: Snapshot, now: i64) -> Result<Manifest> {
        let Snapshot {
            label,
            game_build,
            files,
            archives,
            links,
        } = snapshot;
        let number = match base {
            true => 0,
            false => self.versions().last().map_or(1, |m| m.number + 1),
        };
        let folder = self.folder(number);
        if base && folder.join("manifest.json").exists() {
            return Err(PailuneError::Invalid("a base backup already exists".into()));
        }
        files
            .iter()
            .try_for_each(|(rel, bytes)| write_atomic(&folder.join("files").join(rel), bytes))?;
        let manifest = Manifest {
            number,
            created_at: now,
            label,
            game_build,
            files: files
                .iter()
                .map(|(rel, bytes)| Stored {
                    rel: rel.clone(),
                    sha256: sha256(bytes),
                })
                .collect(),
            archives,
            links,
        };
        // The manifest last: a folder without one is an interrupted save.
        write_atomic(
            &folder.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)?,
        )?;
        Ok(manifest)
    }

    /// Stores one of Pailune's archives by content; returns its hash.
    pub fn put_archive(&self, bytes: &[u8]) -> Result<String> {
        let hash = sha256(bytes);
        let path = self.archive_path(&hash);
        if !path.exists() {
            write_atomic(&path, bytes)?;
        }
        Ok(hash)
    }

    /// Where an archive with this content hash is kept.
    pub fn archive_path(&self, hash: &str) -> PathBuf {
        self.root.join("archives").join(format!("{hash}.paz"))
    }

    /// The stored bytes of one file, checked against the manifest.
    pub fn read(&self, manifest: &Manifest, rel: &str) -> Result<Vec<u8>> {
        let stored = manifest
            .files
            .iter()
            .find(|f| f.rel == rel)
            .ok_or_else(|| PailuneError::Invalid(format!("{rel} is not in this backup")))?;
        let path = self.folder(manifest.number).join("files").join(rel);
        let bytes = fs::read(&path).at(&path)?;
        match sha256(&bytes) == stored.sha256 {
            true => Ok(bytes),
            false => Err(PailuneError::Invalid(format!(
                "backup copy of {rel} is damaged"
            ))),
        }
    }

    /// Puts every file of the base (0) or a version back into `game_dir`.
    /// All copies are read and checked before the first one is written.
    pub fn restore(&self, number: u32, game_dir: &Path) -> Result<Manifest> {
        let manifest = self.manifest(number)?;
        let files = manifest
            .files
            .iter()
            .map(|f| Ok((f.rel.clone(), self.read(&manifest, &f.rel)?)))
            .collect::<Result<Vec<_>>>()?;
        files
            .iter()
            .try_for_each(|(rel, bytes)| write_atomic(&game_dir.join(rel), bytes))?;
        Ok(manifest)
    }

    /// Which stored state the game folder is in: the base (0), a version,
    /// or `None` when something else changed the files.
    pub fn live_state(&self, game_dir: &Path) -> Result<Option<u32>> {
        let base = self.manifest(0)?;
        let matches = |m: &Manifest| {
            m.files
                .iter()
                .all(|f| fs::read(game_dir.join(&f.rel)).is_ok_and(|b| sha256(&b) == f.sha256))
        };
        let latest_first = self.versions().into_iter().rev();
        Ok(latest_first
            .chain(std::iter::once(base))
            .find(|m| matches(m))
            .map(|m| m.number))
    }
}
