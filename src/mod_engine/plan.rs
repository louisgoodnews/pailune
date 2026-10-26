//! Maps staged mod files onto their destination inside the game directory.

use super::model::Registry;
pub use super::placement::{
    PazFormat, Placement, is_archive_dir, placement, text_target, virtual_path,
};
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
/// Only plain relative paths qualify: nothing a mod ships may be linked
/// outside the game folder (`..`, root or drive prefixes are refused).
fn destination(config: &Config, rel: &Path, place: Placement) -> Option<PathBuf> {
    let plain = rel
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)));
    if !plain {
        tracing::warn!(path = %rel.display(), "refusing a mod path outside the game folder");
        return None;
    }
    let game = &config.game.dir;
    let inside = match place {
        Placement::Mirror => Some(rel.to_path_buf()),
        Placement::Bin64 => rel.file_name().map(|name| Path::new("bin64").join(name)),
        // Packed and prebuilt content reaches the game through the overlay.
        Placement::Pack
        | Placement::PrebuiltGroup
        | Placement::JsonPatch
        | Placement::TextPatch
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

/// The mod's FOMOD installer; `Err` with the reason when it is broken.
pub fn installer(mod_root: &Path) -> std::result::Result<Option<crate::fomod::Config>, String> {
    crate::fomod::load(mod_root).map_err(|err| err.to_string())
}

/// A deployable file of a staged mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFile {
    /// The staged file.
    pub source: PathBuf,
    /// Path in the game layout (relative to the mod root, without the
    /// variant folder).
    pub rel: PathBuf,
    pub place: Placement,
}

/// Every deployable file of a mod. FOMOD mods deploy what their installer
/// resolves to for the stored choices; for multi-variant mods only the
/// chosen variant (default: the first) and the shared root files are included.
/// A mod with a broken installer deploys nothing: its folders hold every
/// option at once, so deploying them as they are would mix alternatives. It
/// is logged and shown in the details panel instead of blocking every Apply.
pub fn deployable_files(mod_root: &Path, variant: Option<&str>) -> Result<Vec<ModFile>> {
    let installer = match installer(mod_root) {
        Ok(installer) => installer,
        Err(err) => {
            tracing::warn!(mod_root = %mod_root.display(), %err, "not deploying a mod with a broken FOMOD installer");
            return Ok(Vec::new());
        }
    };
    if let Some(installer) = installer {
        let selection = crate::fomod::Selection::decode(variant);
        return Ok(
            crate::fomod::resolve(mod_root, &installer, selection.as_ref())
                .into_iter()
                .map(|(source, rel)| ModFile {
                    place: placement(&rel),
                    source,
                    rel,
                })
                .filter(|f| f.place != Placement::Skip)
                .collect(),
        );
    }
    let options = super::variants::variants(mod_root);
    let chosen = super::variants::effective(&options, variant).cloned();
    WalkDir::new(mod_root)
        .sort_by_file_name()
        .into_iter()
        .filter(|entry| entry.as_ref().map_or(true, |e| e.file_type().is_file()))
        .map(|entry| {
            let source = entry?.into_path();
            let full = source
                .strip_prefix(mod_root)
                .unwrap_or(&source)
                .to_path_buf();
            // Option folders only hold files (a root file is never one).
            let first = full
                .components()
                .next()
                .filter(|_| full.components().count() > 1)
                .map(|c| c.as_os_str().to_string_lossy().into_owned());
            // Alternative JSON files are options by their relative path.
            let key = full.to_string_lossy().replace('\\', "/");
            let rel = match first.filter(|f| options.contains(f)) {
                Some(f) if Some(&f) == chosen.as_ref() => {
                    Some(full.strip_prefix(&f).unwrap_or(&full).to_path_buf())
                }
                Some(_) => None,
                None if options.contains(&key) && chosen.as_ref() != Some(&key) => None,
                None => Some(full),
            };
            Ok(rel.map(|rel| ModFile {
                place: placement(&rel),
                source,
                rel,
            }))
        })
        .filter_map(|item: Result<Option<ModFile>>| item.transpose())
        .filter(|item| {
            !matches!(
                item,
                Ok(ModFile {
                    place: Placement::Skip,
                    ..
                })
            )
        })
        .collect()
}

/// Every link one mod would create, before conflict resolution.
pub fn mod_ops(
    config: &Config,
    staging: &Path,
    name: &str,
    variant: Option<&str>,
) -> Result<Vec<LinkOp>> {
    Ok(deployable_files(&staging.join(name), variant)?
        .into_iter()
        .filter_map(|f| {
            destination(config, &f.rel, f.place).map(|target| LinkOp {
                source: f.source,
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
        .map(|m| mod_ops(config, staging, &m.name, m.variant.as_deref()))
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
