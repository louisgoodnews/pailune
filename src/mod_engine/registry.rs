//! Pure transformations of the mod [`Registry`] plus staging reconciliation.

use super::classify::classify;
use super::model::{ModEntry, MoveTo, Registry};
use super::plan::deployable_files;
use crate::error::{IoContext, Result};
use std::fs;
use std::path::Path;

/// Inspects a staged mod and refreshes `kind` and `file_count` of `base`.
pub fn describe(staging: &Path, base: ModEntry) -> Result<ModEntry> {
    let files = deployable_files(&staging.join(&base.name))?;
    Ok(ModEntry {
        kind: classify(files.iter().map(|(_, place)| *place)),
        file_count: files.len(),
        ..base
    })
}

/// A disabled entry for a mod found in staging without a database row.
fn discovered(name: &str, now: i64) -> ModEntry {
    ModEntry {
        name: name.to_string(),
        kind: super::model::ModKind::Empty,
        enabled: false,
        file_count: 0,
        installed_at: now,
        source: String::new(),
        nexus: None,
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

/// Reconciles `registry` with the staging folder.
///
/// Known mods keep their order, flags and metadata; vanished mods are
/// dropped; newly found folders are appended disabled.
pub fn sync(registry: &Registry, staging: &Path, now: i64) -> Result<Registry> {
    let names = staged_names(staging)?;
    let known = registry
        .mods
        .iter()
        .filter(|m| names.contains(&m.name))
        .cloned();
    let fresh = names
        .iter()
        .filter(|n| registry.mods.iter().all(|m| &m.name != *n))
        .map(|n| discovered(n, now));
    Ok(Registry {
        mods: known
            .chain(fresh)
            .map(|m| describe(staging, m))
            .collect::<Result<_>>()?,
    })
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
    };
    let mut mods = registry.mods.clone();
    let entry = mods.remove(from);
    mods.insert(target, entry);
    Registry { mods }
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
mod tests {
    use super::*;

    fn reg(names: &[&str]) -> Registry {
        Registry {
            mods: names.iter().map(|n| discovered(n, 0)).collect(),
        }
    }

    fn order(r: &Registry) -> Vec<&str> {
        r.mods.iter().map(|m| m.name.as_str()).collect()
    }

    #[test]
    fn moves_and_flags() {
        let r = reg(&["a", "b", "c"]);
        assert_eq!(order(&moved(&r, "c", MoveTo::Top)), ["c", "a", "b"]);
        assert_eq!(order(&moved(&r, "a", MoveTo::Bottom)), ["b", "c", "a"]);
        assert_eq!(order(&moved(&r, "a", MoveTo::Up)), ["a", "b", "c"]);
        assert_eq!(order(&moved(&r, "b", MoveTo::Down)), ["a", "c", "b"]);
        assert_eq!(order(&without(&r, "b")), ["a", "c"]);
        assert!(with_all(&r, true).mods.iter().all(|m| m.enabled));
        assert!(with_enabled(&r, "b", true).mods[1].enabled);
    }
}
