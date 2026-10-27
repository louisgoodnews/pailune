//! Portable mod lists (board item B-12): the load order, enabled flags,
//! chosen options and origins of every mod, as a JSON file that can be
//! shared and imported to rebuild the same setup elsewhere.
//!
//! Unlike profiles, a mod list carries enough to *reinstall* missing mods:
//! the Nexus mod and file ids, and the collections a mod belongs to.

pub mod apply;

use crate::error::{IoContext, PailuneError, Result};
use crate::mod_engine::model::{NexusRef, Registry};
use crate::nexus::collection::Collection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Marker in every file, so other JSON is rejected early.
pub const KIND: &str = "pailune-modlist";

/// Version of the layout.
pub const FORMAT: u32 = 1;

/// Nexus origin of a listed mod.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListedNexus {
    pub mod_id: u64,
    pub file_id: u64,
    #[serde(default)]
    pub version: String,
}

/// One mod in load order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListedMod {
    pub name: String,
    pub enabled: bool,
    /// Chosen option folder of a multi-variant mod.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// Where it was installed from (file name or Nexus link), for manual reinstalls.
    #[serde(default)]
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nexus: Option<ListedNexus>,
    /// Slugs of the installed collections this mod belongs to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<String>,
}

/// A whole mod list file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModList {
    pub kind: String,
    pub format: u32,
    /// Nexus game domain the list was made for.
    pub game: String,
    pub exported_at: i64,
    pub app_version: String,
    /// Mods in load order (later wins).
    pub mods: Vec<ListedMod>,
}

impl ListedNexus {
    /// From a stored Nexus origin.
    fn from_ref(origin: &NexusRef) -> Self {
        Self {
            mod_id: origin.mod_id,
            file_id: origin.file_id,
            version: origin.version.clone(),
        }
    }
}

/// Captures the registry (and collection membership) as a mod list.
pub fn capture(registry: &Registry, collections: &[Collection], game: &str, now: i64) -> ModList {
    let member_of = |name: &str| {
        collections
            .iter()
            .filter(|c| {
                c.members
                    .iter()
                    .any(|m| m.mod_name.as_deref() == Some(name))
            })
            .map(|c| c.slug.clone())
            .collect()
    };
    ModList {
        kind: KIND.into(),
        format: FORMAT,
        game: game.into(),
        exported_at: now,
        app_version: env!("CARGO_PKG_VERSION").into(),
        mods: registry
            .mods
            .iter()
            .map(|m| ListedMod {
                name: m.name.clone(),
                enabled: m.enabled,
                variant: m.variant.clone(),
                source: m.source.clone(),
                nexus: m.nexus.as_ref().map(ListedNexus::from_ref),
                collections: member_of(&m.name),
            })
            .collect(),
    }
}

/// Writes a mod list as pretty JSON.
pub fn write(path: &Path, list: &ModList) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(list)?).at(path)
}

/// Reads and checks a mod list file.
pub fn read(path: &Path, game: &str) -> Result<ModList> {
    let list: ModList = serde_json::from_str(&fs::read_to_string(path).at(path)?)
        .map_err(|e| PailuneError::Invalid(format!("not a Pailune mod list: {e}")))?;
    match (list.kind == KIND, list.format <= FORMAT, list.game == game) {
        (false, _, _) => Err(PailuneError::Invalid("not a Pailune mod list".into())),
        (_, false, _) => Err(PailuneError::Invalid(
            "this mod list was made by a newer Pailune; please update".into(),
        )),
        (_, _, false) => Err(PailuneError::Invalid(format!(
            "this mod list is for '{}', not '{game}'",
            list.game
        ))),
        _ => Ok(list),
    }
}

#[cfg(test)]
mod tests;
