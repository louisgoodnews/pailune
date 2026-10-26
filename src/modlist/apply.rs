//! Applying a mod list to the installed mods (pure).

use super::{ListedMod, ModList};
use crate::mod_engine::model::{ModEntry, Registry};

/// Result of applying a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Installed mods in list order with the list's flags and options;
    /// mods the list does not know follow, disabled.
    pub registry: Registry,
    /// Listed mods that were found.
    pub matched: usize,
    /// Listed mods that are not installed, in list order.
    pub missing: Vec<ListedMod>,
}

/// How sure a match is, strongest first: the same Nexus file, the same Nexus
/// mod under the same name (an updated file), the same name, the same Nexus
/// mod. A Nexus page with several files installed stays one-to-one this way.
const TIERS: [fn(&ListedMod, &ModEntry) -> bool; 4] = [
    |l, m| nexus(l, m, |a, b| a.mod_id == b.mod_id && a.file_id == b.file_id),
    |l, m| l.name == m.name && nexus(l, m, |a, b| a.mod_id == b.mod_id),
    |l, m| l.name == m.name,
    |l, m| nexus(l, m, |a, b| a.mod_id == b.mod_id),
];

/// Whether both carry a Nexus origin and `same` holds for them.
fn nexus(
    listed: &ListedMod,
    installed: &ModEntry,
    same: fn(&super::ListedNexus, &crate::mod_engine::model::NexusRef) -> bool,
) -> bool {
    listed
        .nexus
        .as_ref()
        .zip(installed.nexus.as_ref())
        .is_some_and(|(a, b)| same(a, b))
}

/// The installed mod for every listed one. Each tier runs over the whole
/// list before the next, so a weak match never takes a mod that a later
/// entry matches strongly; each installed mod matches at most once.
fn assign(list: &[ListedMod], mods: &[ModEntry]) -> Vec<Option<usize>> {
    let mut taken = vec![false; mods.len()];
    let mut found = vec![None; list.len()];
    TIERS.iter().for_each(|matches| {
        list.iter().enumerate().for_each(|(l, listed)| {
            if found[l].is_some() {
                return;
            }
            found[l] = (0..mods.len()).find(|&i| !taken[i] && matches(listed, &mods[i]));
            found[l].iter().for_each(|&i| taken[i] = true);
        });
    });
    found
}

/// Reorders, enables and picks options as the list says.
pub fn apply(list: &ModList, registry: &Registry) -> Outcome {
    let mods = &registry.mods;
    let found = assign(&list.mods, mods);
    let mut taken = vec![false; mods.len()];
    let mut ordered = Vec::new();
    let mut missing = Vec::new();
    list.mods
        .iter()
        .zip(&found)
        .for_each(|(listed, found)| match *found {
            Some(i) => {
                taken[i] = true;
                ordered.push(ModEntry {
                    enabled: listed.enabled,
                    variant: listed.variant.clone().or_else(|| mods[i].variant.clone()),
                    ..mods[i].clone()
                });
            }
            None => missing.push(listed.clone()),
        });
    let matched = ordered.len();
    let rest = mods
        .iter()
        .zip(&taken)
        .filter(|(_, t)| !**t)
        .map(|(m, _)| ModEntry {
            enabled: false,
            ..m.clone()
        });
    ordered.extend(rest);
    Outcome {
        registry: Registry { mods: ordered },
        matched,
        missing,
    }
}

/// After installs while a list is pending: places only the listed mods that
/// were missing and are now installed, after the listed mod before them, and
/// leaves every other mod as the user has it now.
pub fn place_new(list: &ModList, registry: &Registry, missing: &[ListedMod]) -> Outcome {
    let found = assign(&list.mods, &registry.mods);
    let name = |l: usize| found[l].map(|i| registry.mods[i].name.clone());
    let mut mods = registry.mods.clone();
    let mut matched = 0;
    let mut still = Vec::new();
    list.mods.iter().enumerate().for_each(|(l, listed)| {
        if !missing.contains(listed) {
            return;
        }
        let Some(new) = name(l) else {
            still.push(listed.clone());
            return;
        };
        let Some(at) = mods.iter().position(|m| m.name == new) else {
            return;
        };
        let entry = mods.remove(at);
        let after = (0..l)
            .rev()
            .filter_map(name)
            .find_map(|n| mods.iter().position(|m| m.name == n));
        mods.insert(
            after.map_or(0, |a| a + 1),
            ModEntry {
                enabled: listed.enabled,
                variant: listed.variant.clone().or(entry.variant.clone()),
                ..entry
            },
        );
        matched += 1;
    });
    Outcome {
        registry: Registry { mods },
        matched,
        missing: still,
    }
}
