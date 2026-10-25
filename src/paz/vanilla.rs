//! The unmodified game archives, as the base for building an overlay.

use super::papgt::{GroupEntry, Papgt};
use super::{pamt, papgt};
use crate::error::{IoContext, PyluneError, Result};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

/// Default for files the game has no counterpart for: LZ4, unencrypted.
const FALLBACK_FLAGS: u16 = 0x0002;

/// Which vanilla files a build needs to know about.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Wanted {
    /// Exact virtual paths.
    pub paths: BTreeSet<String>,
    /// File names to find anywhere (for patches naming only a basename).
    pub names: BTreeSet<String>,
}

/// Vanilla group table plus the storage flags Pylune must reuse.
#[derive(Debug, Clone)]
pub struct Vanilla {
    pub papgt: Papgt,
    /// Wanted files: virtual path → (group name, record). First group wins.
    located: HashMap<String, (String, pamt::FileRecord)>,
    /// Flags of the vanilla entries the overlay replaces.
    flags: HashMap<String, u16>,
    /// Most common flags per file extension, for brand-new files.
    by_extension: HashMap<String, u16>,
}

fn extension(vpath: &str) -> String {
    vpath
        .rsplit_once('.')
        .map(|(_, e)| e.to_string())
        .unwrap_or_default()
}

/// Parses the vanilla table and checks it against the archives on disk:
/// every present group's index must match the checksum the table records.
/// Only then is it safe to base a rebuilt table on it.
pub fn load(game_dir: &Path, papgt_bytes: &[u8], wanted: &Wanted) -> Result<Vanilla> {
    let table = papgt::parse(papgt_bytes)?;
    let mut flags = HashMap::new();
    let mut located = HashMap::new();
    let mut counts: HashMap<(String, u16), usize> = HashMap::new();
    table.groups.iter().try_for_each(|group| {
        let path = game_dir.join(&group.name).join("0.pamt");
        let data = match (fs::read(&path), group.is_optional) {
            (Ok(data), _) => data,
            (Err(_), true) => return Ok(()),
            (Err(err), false) => return Err(err).at(&path),
        };
        let index = pamt::parse(&data)?;
        if index.crc != group.pamt_crc {
            return Err(PyluneError::Invalid(format!(
                "the metadata backup does not match archive group {} — the game was probably \
                 updated; refresh the backup first",
                group.name
            )));
        }
        index.files.iter().for_each(|(vpath, record)| {
            *counts.entry((extension(vpath), record.flags)).or_default() += 1;
            if wanted.paths.contains(vpath) {
                // First group in table order wins, like the game.
                flags.entry(vpath.clone()).or_insert(record.flags);
            }
            let name = vpath.rsplit('/').next().unwrap_or(vpath);
            if wanted.paths.contains(vpath) || wanted.names.contains(name) {
                located
                    .entry(vpath.clone())
                    .or_insert_with(|| (group.name.clone(), *record));
            }
        });
        Ok(())
    })?;
    let by_extension = counts.into_iter().fold(
        HashMap::<String, (u16, usize)>::new(),
        |mut best, ((ext, f), n)| {
            let entry = best.entry(ext).or_insert((f, 0));
            if n > entry.1 {
                *entry = (f, n);
            }
            best
        },
    );
    Ok(Vanilla {
        papgt: table,
        located,
        flags,
        by_extension: by_extension.into_iter().map(|(e, (f, _))| (e, f)).collect(),
    })
}

impl Vanilla {
    /// Storage flags for a file: the shadowed vanilla entry's, else the
    /// usual flags of its extension, else plain LZ4.
    pub fn flags_for(&self, vpath: &str) -> u16 {
        self.flags
            .get(vpath)
            .or_else(|| self.located.get(vpath).map(|(_, record)| &record.flags))
            .or_else(|| self.by_extension.get(&extension(vpath)))
            .copied()
            .unwrap_or(FALLBACK_FLAGS)
    }

    /// Whether the game ships a file at this path (i.e. it is replaced).
    pub fn replaces(&self, vpath: &str) -> bool {
        self.flags.contains_key(vpath)
    }

    /// Group and record of a wanted vanilla file.
    pub fn locate(&self, vpath: &str) -> Option<&(String, pamt::FileRecord)> {
        self.located.get(vpath)
    }

    /// Located virtual paths whose file name is `name`.
    pub fn with_name(&self, name: &str) -> Vec<&String> {
        let mut paths: Vec<&String> = self
            .located
            .keys()
            .filter(|p| p.rsplit('/').next() == Some(name))
            .collect();
        paths.sort();
        paths
    }

    /// Names already used by the game, which overlays must never take.
    pub fn group_names(&self) -> BTreeSet<String> {
        self.papgt.groups.iter().map(|g| g.name.clone()).collect()
    }
}

/// The vanilla table with `overlays` registered in front (highest priority
/// first). New entries copy the language mask of the "all languages" group.
pub fn register(vanilla: &Papgt, overlays: &[(String, u32)]) -> Papgt {
    let lang_mask = vanilla
        .groups
        .iter()
        .find(|g| g.name == "0000")
        .map_or(0x7FFF, |g| g.lang_mask);
    let added = overlays.iter().map(|(name, crc)| GroupEntry {
        is_optional: false,
        lang_mask,
        name: name.clone(),
        pamt_crc: *crc,
    });
    Papgt {
        groups: added.chain(vanilla.groups.iter().cloned()).collect(),
        ..vanilla.clone()
    }
}
