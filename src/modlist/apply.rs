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

/// Index of the installed mod for a listed one: same Nexus mod first (the
/// file may have been updated), else the same name; each installed mod
/// matches at most once.
fn find(listed: &ListedMod, mods: &[ModEntry], taken: &[bool]) -> Option<usize> {
    let free = |i: &usize| !taken[*i];
    let by_nexus = listed.nexus.as_ref().and_then(|n| {
        (0..mods.len()).filter(free).find(|i| {
            mods[*i]
                .nexus
                .as_ref()
                .is_some_and(|m| m.mod_id == n.mod_id)
        })
    });
    by_nexus.or_else(|| {
        (0..mods.len())
            .filter(free)
            .find(|i| mods[*i].name == listed.name)
    })
}

/// Reorders, enables and picks options as the list says.
pub fn apply(list: &ModList, registry: &Registry) -> Outcome {
    let mods = &registry.mods;
    let mut taken = vec![false; mods.len()];
    let mut ordered = Vec::new();
    let mut missing = Vec::new();
    list.mods
        .iter()
        .for_each(|listed| match find(listed, mods, &taken) {
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
