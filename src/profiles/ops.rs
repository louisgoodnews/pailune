//! Pure transformations on [`ProfileBook`]s and registries.

use super::{LoadOutcome, Profile, ProfileBook, ProfileSlot};
use crate::error::{PyluneError, Result};
use crate::mod_engine::model::{ModEntry, Registry};

/// Captures the registry's current selection and order as a profile.
pub fn capture(name: &str, registry: &Registry) -> Profile {
    Profile {
        name: name.trim().to_string(),
        slots: registry
            .mods
            .iter()
            .map(|m| ProfileSlot {
                name: m.name.clone(),
                enabled: m.enabled,
            })
            .collect(),
    }
}

/// Inserts or replaces `profile` (matched by name) and marks it active.
pub fn upsert(book: &ProfileBook, profile: Profile) -> Result<ProfileBook> {
    if profile.name.is_empty() {
        return Err(PyluneError::Invalid(
            "profile name must not be empty".into(),
        ));
    }
    let active = Some(profile.name.clone());
    let mut profiles: Vec<Profile> = book
        .profiles
        .iter()
        .filter(|p| Some(&p.name) != active.as_ref())
        .cloned()
        .chain(std::iter::once(profile))
        .collect();
    profiles.sort_by_key(|p| p.name.to_lowercase());
    Ok(ProfileBook { profiles, active })
}

/// Removes the profile called `name`.
pub fn delete(book: &ProfileBook, name: &str) -> ProfileBook {
    ProfileBook {
        profiles: book
            .profiles
            .iter()
            .filter(|p| p.name != name)
            .cloned()
            .collect(),
        active: book.active.clone().filter(|a| a != name),
    }
}

/// Finds a profile by name.
pub fn find<'a>(book: &'a ProfileBook, name: &str) -> Result<&'a Profile> {
    book.profiles
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| PyluneError::Invalid(format!("no profile named '{name}'")))
}

/// Reorders and re-flags `registry` according to `profile`.
///
/// Mods in the profile come first in profile order; staged mods the profile
/// does not know are appended disabled. Unknown profile entries are reported.
pub fn apply(profile: &Profile, registry: &Registry) -> (Registry, LoadOutcome) {
    let lookup = |name: &str| registry.mods.iter().find(|m| m.name == name);
    let listed = profile.slots.iter().filter_map(|slot| {
        lookup(&slot.name).map(|m| ModEntry {
            enabled: slot.enabled,
            ..m.clone()
        })
    });
    let unlisted = registry
        .mods
        .iter()
        .filter(|m| profile.slots.iter().all(|s| s.name != m.name))
        .map(|m| ModEntry {
            enabled: false,
            ..m.clone()
        });
    let missing = profile
        .slots
        .iter()
        .filter(|s| lookup(&s.name).is_none())
        .map(|s| s.name.clone())
        .collect();
    (
        Registry {
            mods: listed.chain(unlisted).collect(),
        },
        LoadOutcome { missing },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mod_engine::model::ModKind;

    fn entry(name: &str, enabled: bool) -> ModEntry {
        ModEntry {
            name: name.into(),
            kind: ModKind::Asi,
            enabled,
            file_count: 1,
        }
    }

    #[test]
    fn round_trip_reorders_and_reports_missing() {
        let reg = Registry {
            mods: vec![entry("a", true), entry("b", false), entry("c", true)],
        };
        let mut profile = capture("Main", &reg);
        profile.slots.reverse();
        profile.slots.push(ProfileSlot {
            name: "gone".into(),
            enabled: true,
        });
        let book = upsert(&ProfileBook::default(), profile).unwrap();
        let (next, outcome) = apply(find(&book, "Main").unwrap(), &reg);
        let order: Vec<_> = next
            .mods
            .iter()
            .map(|m| (m.name.as_str(), m.enabled))
            .collect();
        assert_eq!(order, [("c", true), ("b", false), ("a", true)]);
        assert_eq!(outcome.missing, ["gone"]);
        assert!(delete(&book, "Main").profiles.is_empty());
        assert!(upsert(&book, capture("  ", &reg)).is_err());
    }
}
