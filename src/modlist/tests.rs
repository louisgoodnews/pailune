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
