//! Pure transformations of the mod [`Registry`] plus staging reconciliation.

use super::classify::classify;
use super::model::{ModEntry, ModKind, MoveTo, Registry, Scan, Scanned};
use super::plan::{Placement, deployable_files};
use crate::error::{IoContext, Result};
use std::fs;
use std::path::Path;

/// Classifies one staged mod. Unreadable folders count as empty (with a
/// warning in the log) instead of failing the whole scan.
fn scan_one(staging: &Path, name: &str) -> Scanned {
    match deployable_files(&staging.join(name), None) {
        Ok(files) => {
            let paz_files = files
                .iter()
                .filter(|f| matches!(f.place, Placement::NeedsPaz(_)))
                .count();
            Scanned {
                kind: classify(files.iter().map(|f| f.place)),
                file_count: files.len() - paz_files,
                paz_files,
            }
        }
        Err(err) => {
            tracing::warn!(mod_name = name, %err, "could not scan staged mod");
            Scanned {
                kind: ModKind::Empty,
                file_count: 0,
                paz_files: 0,
            }
        }
    }
}

/// Classifies every staged mod. Runs outside any database transaction.
pub fn scan(staging: &Path) -> Result<Scan> {
    Ok(staged_names(staging)?
        .into_iter()
        .map(|n| {
            let scanned = scan_one(staging, &n);
            (n, scanned)
        })
        .collect())
}

/// A disabled entry for a mod found in staging without a database row.
pub(crate) fn discovered(name: &str, now: i64) -> ModEntry {
    ModEntry {
        name: name.to_string(),
        kind: ModKind::Empty,
        enabled: false,
        file_count: 0,
        paz_files: 0,
        installed_at: now,
        source: String::new(),
        nexus: None,
        variant: None,
    }
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

/// Reconciles `registry` with a staging [`Scan`] (pure).
///
/// Known mods keep their order, flags and metadata; vanished mods are
/// dropped; newly found folders are appended disabled.
pub fn merge(registry: &Registry, scan: &Scan, now: i64) -> Registry {
    let refresh = |m: ModEntry| {
        scan.get(&m.name).map(|s| ModEntry {
            kind: s.kind,
            file_count: s.file_count,
            paz_files: s.paz_files,
            ..m
        })
    };
    let known = registry.mods.iter().cloned().filter_map(refresh);
    let fresh = scan
        .iter()
        .filter(|(n, _)| registry.mods.iter().all(|m| &m.name != *n))
        .map(|(n, s)| ModEntry {
            kind: s.kind,
            file_count: s.file_count,
            paz_files: s.paz_files,
            ..discovered(n, now)
        });
    Registry {
        mods: known.chain(fresh).collect(),
    }
}

/// [`scan`] followed by [`merge`] (tests; production code scans before
/// opening the write transaction).
#[cfg(test)]
pub fn sync(registry: &Registry, staging: &Path, now: i64) -> Result<Registry> {
    Ok(merge(registry, &scan(staging)?, now))
}

/// Maps the entry called `name` through `f`, leaving the others untouched.
fn map_named(registry: &Registry, name: &str, f: impl Fn(&ModEntry) -> ModEntry) -> Registry {
    Registry {
        mods: registry
            .mods
            .iter()
            .map(|m| if m.name == name { f(m) } else { m.clone() })
            .collect(),
    }
}

/// Sets the enabled flag of one mod.
pub fn with_enabled(registry: &Registry, name: &str, enabled: bool) -> Registry {
    map_named(registry, name, |m| ModEntry {
        enabled,
        ..m.clone()
    })
}

/// Enables or disables every mod.
pub fn with_all(registry: &Registry, enabled: bool) -> Registry {
    Registry {
        mods: registry
            .mods
            .iter()
            .map(|m| ModEntry {
                enabled,
                ..m.clone()
            })
            .collect(),
    }
}

/// Moves one mod within the load order.
pub fn moved(registry: &Registry, name: &str, to: MoveTo) -> Registry {
    let Some(from) = registry.mods.iter().position(|m| m.name == name) else {
        return registry.clone();
    };
    let last = registry.mods.len() - 1;
    let target = match to {
        MoveTo::Top => 0,
        MoveTo::Up => from.saturating_sub(1),
        MoveTo::Down => (from + 1).min(last),
        MoveTo::Bottom => last,
        // 1-based, as shown in the list; out-of-range numbers clamp to the ends.
        MoveTo::Position(number) => number.saturating_sub(1).min(last),
    };
    let mut mods = registry.mods.clone();
    let entry = mods.remove(from);
    mods.insert(target, entry);
    Registry { mods }
}

/// Places the named mods (as far as staged) together in the given order,
/// starting where the first of them was, and enables them. Used for the
/// curator's load order of a collection.
pub fn arranged(registry: &Registry, names: &[String]) -> Registry {
    let listed = |m: &&ModEntry| names.contains(&m.name);
    let start = registry
        .mods
        .iter()
        .position(|m| names.contains(&m.name))
        .unwrap_or(registry.mods.len());
    let group = names
        .iter()
        .filter_map(|n| registry.mods.iter().find(|m| &m.name == n))
        .map(|m| ModEntry {
            enabled: true,
            ..m.clone()
        });
    let rest: Vec<ModEntry> = registry
        .mods
        .iter()
        .filter(|m| !listed(m))
        .cloned()
        .collect();
    let before = registry.mods[..start]
        .iter()
        .filter(|m| !names.contains(&m.name))
        .count();
    let mods = rest[..before]
        .iter()
        .cloned()
        .chain(group)
        .chain(rest[before..].iter().cloned())
        .collect();
    Registry { mods }
}

/// Chooses the option folder of a multi-variant mod.
pub fn with_variant(registry: &Registry, name: &str, variant: Option<String>) -> Registry {
    map_named(registry, name, |m| ModEntry {
        variant: variant.clone(),
        ..m.clone()
    })
}

/// The registry without the mod called `name`.
pub fn without(registry: &Registry, name: &str) -> Registry {
    Registry {
        mods: registry
            .mods
            .iter()
            .filter(|m| m.name != name)
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests;
