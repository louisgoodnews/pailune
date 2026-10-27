//! The player's save slots, summarised from their `lobby.save` (read-only).

use super::body::{self, walk};
use super::view::{Node, find, tree};
use crate::error::{IoContext, PailuneError, Result};
use std::path::{Path, PathBuf};

/// One save slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// Folder name, e.g. `slot1` (100 and up are automatic saves).
    pub name: String,
    pub dir: PathBuf,
    pub level: Option<String>,
    /// Unix seconds of the save, from the slot summary.
    pub saved_at: Option<i64>,
    pub mission: Option<String>,
    pub size: u64,
    /// Why the slot could not be read, if it could not.
    pub error: Option<String>,
}

/// Reads one slot summary.
fn slot(dir: &Path) -> Slot {
    let name = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let size = std::fs::metadata(dir.join("save.save"))
        .map(|m| m.len())
        .unwrap_or(0);
    let summary = std::fs::read(dir.join("lobby.save"))
        .map_err(|e| PailuneError::Invalid(e.to_string()))
        .and_then(|bytes| super::decode(&bytes))
        .and_then(|save| {
            let body = body::parse(&save.body)?;
            let lobby = walk::object(&body, &save.body, 0)?;
            Ok((
                find(&body, &lobby, "_level"),
                find(&body, &lobby, "_generatedTime"),
                find(&body, &lobby, "_slotDisplayNameMissionKey"),
            ))
        });
    match summary {
        Ok((level, time, mission)) => Slot {
            name,
            dir: dir.to_path_buf(),
            level,
            saved_at: time.and_then(|t| t.parse().ok()),
            mission,
            size,
            error: None,
        },
        Err(err) => Slot {
            name,
            dir: dir.to_path_buf(),
            level: None,
            saved_at: None,
            mission: None,
            size,
            error: Some(err.to_string()),
        },
    }
}

/// Every slot below the save folder (`<account>/slot<N>/`), by name.
pub fn slots(save_dir: &Path) -> Vec<Slot> {
    let mut found: Vec<Slot> = walkdir::WalkDir::new(save_dir)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.path().join("save.save").is_file())
        .map(|e| slot(e.path()))
        .collect();
    let number = |s: &Slot| {
        s.name
            .trim_start_matches("slot")
            .parse::<u32>()
            .unwrap_or(u32::MAX)
    };
    found.sort_by_key(number);
    found
}

/// The whole `save.save` of a slot as a tree: one node per top-level object.
pub fn inspect(dir: &Path) -> Result<Vec<Node>> {
    let path = dir.join("save.save");
    let save = super::decode(&std::fs::read(&path).at(&path)?)?;
    let body = body::parse(&save.body)?;
    (0..body.toc.len())
        .map(|i| {
            let object = walk::object(&body, &save.body, i)?;
            Ok(Node {
                label: body.types[object.class as usize].name.clone(),
                children: tree(&body, &object),
                edit: None,
            })
        })
        .collect()
}
