//! Tests of collection parsing and update planning.

use super::*;

fn member(
    position: usize,
    mod_id: u64,
    file_id: u64,
    optional: bool,
    name: Option<&str>,
) -> Member {
    Member {
        position,
        mod_id,
        file_id,
        optional,
        selected: !optional,
        title: String::new(),
        size: 0,
        mod_name: name.map(String::from),
    }
}

fn collection(slug: &str, members: Vec<Member>) -> Collection {
    Collection {
        slug: slug.into(),
        name: slug.into(),
        revision: 1,
        members,
    }
}

#[test]
fn parses_graphql_and_slugs() {
    let json: Value = serde_json::from_str(r#"{"data":{"collectionRevision":{"revisionNumber":12,
        "collection":{"name":"Crimson QOL","slug":"j67ryq"},
        "modFiles":[{"fileId":3120,"optional":false,"file":{"modId":618,"name":"RAW","version":"1.4","sizeInBytes":"36831","mod":{"name":"Better Radial Menus"}}},
                    {"fileId":3056,"optional":true,"file":{"modId":120,"name":"Glider","sizeInBytes":"10","mod":{"name":"Lower Glider Stamina"}}}]}}}"#).unwrap();
    let c = from_graphql("j67ryq", &json).unwrap();
    assert_eq!(
        (c.name.as_str(), c.revision, c.members.len()),
        ("Crimson QOL", 12, 2)
    );
    assert_eq!(
        (
            c.members[0].mod_id,
            c.members[0].size,
            c.members[0].selected
        ),
        (618, 36831, true)
    );
    assert!(!c.members[1].selected, "optional members start unselected");
    assert_eq!(
        parse_slug("https://next.nexusmods.com/crimsondesert/collections/J67RYQ?tab=mods")
            .as_deref(),
        Some("j67ryq")
    );
    assert_eq!(parse_slug(" j67ryq ").as_deref(), Some("j67ryq"));
    assert_eq!(parse_slug("../x"), None);
}

#[test]
fn plans_revision_updates() {
    let installed = collection(
        "a",
        vec![
            member(0, 1, 10, false, Some("One")),
            member(1, 2, 20, false, Some("Two")),
            member(2, 3, 30, false, Some("Three")),
            member(3, 4, 40, true, None),
        ],
    );
    let latest = collection(
        "a",
        vec![
            member(0, 1, 10, false, None),
            member(1, 2, 21, false, None),
            member(2, 5, 50, false, None),
            member(3, 6, 60, true, None),
        ],
    );
    let other = collection("b", vec![member(0, 3, 30, false, Some("Three"))]);
    let plan = plan_update(&installed, &latest, &[other]);
    let downloads: Vec<(u64, Option<&str>)> = plan
        .download
        .iter()
        .map(|(m, r)| (m.file_id, r.as_deref()))
        .collect();
    assert_eq!(
        downloads,
        [(21, Some("Two")), (50, None)],
        "changed file replaces; new optional stays out"
    );
    assert!(
        plan.remove.is_empty(),
        "Three is still used by collection b"
    );
    let alone = plan_update(&installed, &latest, &[]);
    assert_eq!(alone.remove, ["Three"]);
    assert_eq!(exclusive_mods(&installed, &[]), ["One", "Two", "Three"]);
}

#[cfg(test)]
mod network {
    /// `cargo test -- --ignored`: reads a public Crimson Desert collection.
    #[test]
    #[ignore = "talks to api.nexusmods.com"]
    fn fetches_a_public_collection() {
        let client = crate::nexus::api::Client::new("", crate::config::NEXUS_DOMAIN);
        let c = client.collection("j67ryq", None).unwrap();
        println!(
            "{} rev {} — {} members, e.g. {:?}",
            c.name,
            c.revision,
            c.members.len(),
            c.members[0].title
        );
        assert!(c.members.len() > 5 && c.members.iter().any(|m| m.optional));
    }
}
