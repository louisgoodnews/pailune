use super::apply::apply;
use super::*;
use crate::mod_engine::model::ModEntry;
use crate::mod_engine::registry::discovered;

fn entry(name: &str, nexus: Option<(u64, u64)>) -> ModEntry {
    ModEntry {
        nexus: nexus.map(|(mod_id, file_id)| NexusRef {
            mod_id,
            file_id,
            version: "1".into(),
        }),
        ..discovered(name, 0)
    }
}

#[test]
fn round_trips_and_applies_by_nexus_id_then_name() {
    let source = Registry {
        mods: vec![
            ModEntry {
                enabled: true,
                variant: Some("Red".into()),
                ..entry("Colours", Some((7, 70)))
            },
            entry("Loose", None),
            ModEntry {
                enabled: true,
                ..entry("Far away", Some((9, 90)))
            },
        ],
    };
    let list = capture(&source, &[], "crimsondesert", 5);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("list.json");
    write(&path, &list).unwrap();
    assert_eq!(read(&path, "crimsondesert").unwrap(), list);
    assert!(read(&path, "othergame").is_err());
    // Elsewhere: "Colours" was renamed and updated, "Far away" is missing,
    // "Extra" is unknown to the list.
    let here = Registry {
        mods: vec![
            entry("Extra", None),
            entry("Loose", None),
            entry("Colours (71)", Some((7, 71))),
        ],
    };
    let outcome = apply(&list, &here);
    let order: Vec<_> = outcome
        .registry
        .mods
        .iter()
        .map(|m| (m.name.as_str(), m.enabled, m.variant.as_deref()))
        .collect();
    assert_eq!(
        order,
        [
            ("Colours (71)", true, Some("Red")),
            ("Loose", false, None),
            ("Extra", false, None),
        ]
    );
    assert_eq!(outcome.matched, 2);
    let missing: Vec<_> = outcome.missing.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(missing, ["Far away"]);
    assert_eq!(outcome.missing[0].nexus.as_ref().unwrap().file_id, 90);
    fs::write(&path, "{\"kind\":\"other\"}").unwrap();
    assert!(read(&path, "crimsondesert").is_err());
}

#[test]
fn several_files_of_one_nexus_mod_match_one_to_one() {
    let listed = |name: &str, file_id| ListedMod {
        name: name.into(),
        enabled: true,
        variant: None,
        source: String::new(),
        nexus: Some(ListedNexus {
            mod_id: 5,
            file_id,
            version: String::new(),
        }),
        collections: Vec::new(),
    };
    // The optional file comes first in the list but second when installed;
    // the main file was updated (new file id, same name).
    let list = ModList {
        mods: vec![listed("Optional", 51), listed("Main", 50)],
        ..capture(&Registry::default(), &[], "crimsondesert", 0)
    };
    let here = Registry {
        mods: vec![
            entry("Main", Some((5, 52))),
            entry("Optional", Some((5, 51))),
        ],
    };
    let names: Vec<_> = apply(&list, &here)
        .registry
        .mods
        .into_iter()
        .map(|m| m.name)
        .collect();
    assert_eq!(names, ["Optional", "Main"]);
}
