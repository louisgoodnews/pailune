//! Keeping the mod [`Registry`] in sync with the staging folder.

use super::classify::classify;
use super::model::{ModEntry, Registry};
use super::plan::deployable_files;
use crate::error::{IoContext, Result};
use std::fs;
use std::path::Path;

/// Inspects a staged mod and builds a fresh [`ModEntry`] for it.
pub fn describe(staging: &Path, name: &str, enabled: bool) -> Result<ModEntry> {
    let files = deployable_files(&staging.join(name))?;
    Ok(ModEntry {
        name: name.to_string(),
        kind: classify(files.iter().map(|(_, place)| *place)),
        enabled,
        file_count: files.len(),
    })
}

/// Lists the mod directory names currently in the staging folder.
fn staged_names(staging: &Path) -> Result<Vec<String>> {
    let mut names = fs::read_dir(staging)
        .at(staging)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

/// Reconciles `registry` with the staging folder.
///
/// Known mods keep their order and enabled flag; vanished mods are dropped;
/// newly found folders are appended disabled.
pub fn sync(registry: &Registry, staging: &Path) -> Result<Registry> {
    let names = staged_names(staging)?;
    let known = registry
        .mods
        .iter()
        .filter(|m| names.contains(&m.name))
        .map(|m| describe(staging, &m.name, m.enabled));
    let fresh = names
        .iter()
        .filter(|n| registry.mods.iter().all(|m| &m.name != *n))
        .map(|n| describe(staging, n, false));
    Ok(Registry {
        mods: known.chain(fresh).collect::<Result<Vec<_>>>()?,
    })
}

/// Returns a registry with the enabled flag of mod `index` flipped.
pub fn toggled(registry: &Registry, index: usize) -> Registry {
    let mods = registry
        .mods
        .iter()
        .enumerate()
        .map(|(i, m)| ModEntry {
            enabled: m.enabled ^ (i == index),
            ..m.clone()
        })
        .collect();
    Registry { mods }
}

/// Returns a registry where mods `a` and `b` swapped places (no-op if invalid).
pub fn swapped(registry: &Registry, a: usize, b: usize) -> Registry {
    let mut mods = registry.mods.clone();
    if a < mods.len() && b < mods.len() {
        mods.swap(a, b);
    }
    Registry { mods }
}

/// Deletes a mod from staging and returns the registry without it.
pub fn removed(registry: &Registry, staging: &Path, index: usize) -> Result<Registry> {
    let Some(entry) = registry.mods.get(index) else {
        return Ok(registry.clone());
    };
    let dir = staging.join(&entry.name);
    fs::remove_dir_all(&dir).at(&dir)?;
    Ok(Registry {
        mods: registry
            .mods
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, m)| m.clone())
            .collect(),
    })
}
