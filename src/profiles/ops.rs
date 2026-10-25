//! Pure transformations on [`ProfileBook`]s and registries.

use super::{LoadOutcome, Profile, ProfileBook, ProfileSlot};
use crate::error::{PyluneError, Result};
use crate::mod_engine::model::{ModEntry, Registry};

/// Captures the registry's current selection and order as a profile.
pub fn capture(name: &str, registry: &Registry, launch_options: &str) -> Profile {
    Profile {
        name: name.trim().to_string(),
        launch_options: launch_options.to_string(),
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
            paz_files: 0,
            installed_at: 0,
            source: String::new(),
            nexus: None,
        }
    }

    #[test]
    fn round_trip_reorders_and_reports_missing() {
        let reg = Registry {
            mods: vec![entry("a", true), entry("b", false), entry("c", true)],
        };
        let mut profile = capture("Main", &reg, "");
        profile.slots.reverse();
        profile.slots.push(ProfileSlot {
            name: "gone".into(),
            enabled: true,
        });
        let book = ProfileBook {
            profiles: vec![profile],
            active: None,
        };
        let (next, outcome) = apply(find(&book, "Main").unwrap(), &reg);
        let order: Vec<_> = next
            .mods
            .iter()
            .map(|m| (m.name.as_str(), m.enabled))
            .collect();
        assert_eq!(order, [("c", true), ("b", false), ("a", true)]);
        assert_eq!(outcome.missing, ["gone"]);
        assert!(find(&book, "Other").is_err());
    }
}

#[cfg(test)]
mod export_tests {
    use crate::profiles::{Profile, ProfileSlot};

    /// Exported profile files are plain JSON and read back unchanged.
    #[test]
    fn profile_files_round_trip() {
        let profile = Profile {
            name: "Main".into(),
            slots: vec![ProfileSlot {
                name: "Camera".into(),
                enabled: true,
            }],
            launch_options: "gamemoderun %command%".into(),
        };
        let text = serde_json::to_string_pretty(&profile).unwrap();
        assert!(text.contains("\"launch_options\""));
        assert_eq!(serde_json::from_str::<Profile>(&text).unwrap(), profile);
    }
}
