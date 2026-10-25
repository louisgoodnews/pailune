//! Maps staged mod files onto their destination inside the game directory.

use super::model::Registry;
pub use super::placement::{PazFormat, Placement, is_archive_dir, placement, virtual_path};
use crate::config::Config;
use crate::error::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A single symlink to create: `target` -> `source`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkOp {
    /// File inside the staging folder.
    pub source: PathBuf,
    /// Location inside the game directory.
    pub target: PathBuf,
}

/// The full set of links for all enabled mods, plus conflict information.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LinkPlan {
    /// Links to create, conflict-free (one per target).
    pub ops: Vec<LinkOp>,
    /// Targets provided by more than one enabled mod.
    pub conflicts: usize,
}

/// Resolves the absolute game path for `rel` under the given placement.
fn destination(config: &Config, rel: &Path, place: Placement) -> Option<PathBuf> {
    let game = &config.game.dir;
    let inside = match place {
        Placement::Mirror => Some(rel.to_path_buf()),
        Placement::Bin64 => rel.file_name().map(|name| Path::new("bin64").join(name)),
        // Packed and prebuilt content reaches the game through the overlay.
        Placement::Pack
        | Placement::PrebuiltGroup
        | Placement::JsonPatch
        | Placement::NeedsPaz(_)
        | Placement::Skip => None,
    };
    inside.map(|inside| resolve_case(game, &inside))
}

/// Joins `rel` onto `base`, reusing existing entries that differ only in
/// letter case. Windows games under Proton see `Bin64` and `bin64` as one
/// directory; on Linux they would be two.
pub fn resolve_case(base: &Path, rel: &Path) -> PathBuf {
    rel.components().fold(base.to_path_buf(), |dir, component| {
        let name = component.as_os_str();
        let wanted = name.to_string_lossy().to_lowercase();
        let existing = match dir.join(name).exists() {
            true => None,
            false => std::fs::read_dir(&dir).ok().and_then(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name())
                    .find(|n| n.to_string_lossy().to_lowercase() == wanted)
            }),
        };
        dir.join(existing.as_deref().unwrap_or(name))
    })
}

/// Lists `(relative path, placement)` for every deployable file in `mod_root`.
pub fn deployable_files(mod_root: &Path) -> Result<Vec<(PathBuf, Placement)>> {
    WalkDir::new(mod_root)
        .sort_by_file_name()
        .into_iter()
        .filter(|entry| entry.as_ref().map_or(true, |e| e.file_type().is_file()))
        .map(|entry| {
            let path = entry?.into_path();
            let rel = path.strip_prefix(mod_root).unwrap_or(&path).to_path_buf();
            let place = placement(&rel);
            Ok((rel, place))
        })
        .filter(|item| !matches!(item, Ok((_, Placement::Skip))))
        .collect()
}

/// Every link one mod would create, before conflict resolution.
pub fn mod_ops(config: &Config, staging: &Path, name: &str) -> Result<Vec<LinkOp>> {
    let root = staging.join(name);
    Ok(deployable_files(&root)?
        .into_iter()
        .filter_map(|(rel, place)| {
            destination(config, &rel, place).map(|target| LinkOp {
                source: root.join(&rel),
                target,
            })
        })
        .collect())
}

/// Builds the link plan for every enabled mod, in load order.
///
/// When two mods provide the same target, the later one wins and the
/// collision is counted in [`LinkPlan::conflicts`].
pub fn build(config: &Config, staging: &Path, registry: &Registry) -> Result<LinkPlan> {
    let candidates = registry
        .mods
        .iter()
        .filter(|m| m.enabled)
        .map(|m| mod_ops(config, staging, &m.name))
        .collect::<Result<Vec<_>>>()?;
    let total: usize = candidates.iter().map(Vec::len).sum();
    let merged: BTreeMap<PathBuf, PathBuf> = candidates
        .into_iter()
        .flatten()
        .map(|op| (op.target, op.source))
        .collect();
    Ok(LinkPlan {
        conflicts: total - merged.len(),
        ops: merged
            .into_iter()
            .map(|(target, source)| LinkOp { source, target })
            .collect(),
    })
}
