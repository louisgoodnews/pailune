//! Maps staged mod files onto their destination inside the game directory.

use super::model::Registry;
use crate::config::Config;
use crate::error::Result;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

/// Where a single staged file belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Same relative path inside the game directory.
    Mirror,
    /// Flattened into `bin64/` (ASI plugins, DLLs, their INIs).
    Bin64,
    /// Into the configured JSON mod directory, keeping its relative path.
    JsonDir,
    /// Documentation, screenshots and other non-game files.
    Skip,
}

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

/// Returns `true` for four-digit archive folder names such as `0036`.
pub fn is_archive_dir(name: &str) -> bool {
    name.len() == 4 && name.bytes().all(|b| b.is_ascii_digit())
}

/// Decides the [`Placement`] of a file from its path relative to the mod root.
pub fn placement(rel: &Path) -> Placement {
    let first = match rel.components().next() {
        Some(Component::Normal(name)) => name.to_string_lossy().to_lowercase(),
        _ => return Placement::Skip,
    };
    let ext = rel
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match (first.as_str(), ext.as_str()) {
        ("bin64" | "meta", _) => Placement::Mirror,
        (dir, _) if is_archive_dir(dir) => Placement::Mirror,
        (_, "asi" | "dll" | "ini") => Placement::Bin64,
        (_, "json") => Placement::JsonDir,
        _ => Placement::Skip,
    }
}

/// Resolves the absolute game path for `rel` under the given placement.
fn destination(config: &Config, rel: &Path, place: Placement) -> Option<PathBuf> {
    let game = &config.game_dir;
    match place {
        Placement::Mirror => Some(game.join(rel)),
        Placement::Bin64 => rel.file_name().map(|name| game.join("bin64").join(name)),
        Placement::JsonDir => Some(game.join(&config.json_subdir).join(rel)),
        Placement::Skip => None,
    }
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

/// Builds the link plan for every enabled mod, in load order.
///
/// When two mods provide the same target, the later one wins and the
/// collision is counted in [`LinkPlan::conflicts`].
pub fn build(config: &Config, staging: &Path, registry: &Registry) -> Result<LinkPlan> {
    let candidates = registry
        .mods
        .iter()
        .filter(|m| m.enabled)
        .map(|m| {
            let root = staging.join(&m.name);
            deployable_files(&root).map(|files| {
                files
                    .into_iter()
                    .filter_map(|(rel, place)| {
                        destination(config, &rel, place).map(|target| LinkOp {
                            source: root.join(&rel),
                            target,
                        })
                    })
                    .collect::<Vec<_>>()
            })
        })
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
