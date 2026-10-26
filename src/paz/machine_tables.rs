//! Read-only checks of the data tables against the locally installed game
//! (`cargo test -- --ignored`): shipped layouts, raw format-3 changes, test
//! mods, and the export for layout research. Nothing is written to the game.

use super::{extract, pamt};
use std::fs;

/// The installed game, found in the Steam libraries.
fn game_dir() -> std::path::PathBuf {
    crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID).expect("game installed")
}

/// Every shipped table layout decodes every record of the installed game.
#[test]
#[ignore = "reads the local game installation"]
fn shipped_table_layouts_match_the_game() {
    let dir = game_dir().join("0008");
    let index = pamt::parse(&fs::read(dir.join("0.pamt")).unwrap()).unwrap();
    let layouts = crate::tables::layouts();
    let mut failures = Vec::new();
    layouts.tables.iter().for_each(|(name, table)| {
        let path = format!("gamedata/binarystaticinfo__/bin/{name}.staticinfobody");
        let head = path.replace("body", "header");
        let read = |p: &str| extract::extract(&dir, p, &index.files[p]).unwrap();
        if let Err(err) = crate::tables::verify(table, &read(&head), &read(&path)) {
            failures.push(err.to_string());
        }
    });
    println!(
        "{} layouts checked, {} failed",
        layouts.tables.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}

/// Exports the installed game's decoded data tables into
/// `PYLUNE_TABLES_OUT=<dir>` for `tools/table-layouts/` (never committed).
#[test]
#[ignore = "reads the local game installation"]
fn export_tables_for_layout_research() {
    let Ok(out) = std::env::var("PYLUNE_TABLES_OUT") else {
        return;
    };
    let dir = game_dir().join("0008");
    let index = pamt::parse(&fs::read(dir.join("0.pamt")).unwrap()).unwrap();
    index
        .files
        .iter()
        .filter(|(p, _)| p.contains("binarystaticinfo__/bin/") && !p.contains("/misc/"))
        .for_each(|(path, record)| {
            let data = extract::extract(&dir, path, record).unwrap();
            let name = path.rsplit('/').next().unwrap();
            fs::write(std::path::Path::new(&out).join(name), data).unwrap();
        });
}

/// A raw format-3 change on a table without a layout (`iteminfo`), in memory.
#[test]
#[ignore = "reads the local game installation"]
fn raw_field_change_on_the_real_item_table() {
    let dir = game_dir().join("0008");
    let index = pamt::parse(&fs::read(dir.join("0.pamt")).unwrap()).unwrap();
    let read = |p: &str| extract::extract(&dir, p, &index.files[p]).unwrap();
    let body = read("gamedata/binarystaticinfo__/bin/iteminfo.staticinfobody");
    let head = read("gamedata/binarystaticinfo__/bin/iteminfo.staticinfoheader");
    let intent = crate::tables::format3::Intent {
        entry: Some("Pyeonjeon_Arrow".into()),
        key: None,
        field: None,
        op: "set".into(),
        old: Some("006400000000000000".into()),
        new: serde_json::json!("00e703000000000000"),
    };
    let (changes, failed) =
        crate::tables::format3::resolve("iteminfo.staticinfobody", &head, &body, &[intent]);
    assert!(failed.is_empty(), "{failed:?}");
    let mut patched = body.clone();
    let patch = crate::patch::FilePatch {
        game_file: "iteminfo".into(),
        signature: None,
        changes,
        fields: Vec::new(),
    };
    let report = crate::patch::apply::apply(&mut patched, &patch, Some(&head), true, false);
    assert!(
        report.failed.is_empty() && report.applied == 1,
        "{report:?}"
    );
    let arrow = crate::tables::records(&head, &patched)
        .unwrap()
        .into_iter()
        .find(|r| r.name == "Pyeonjeon_Arrow")
        .unwrap();
    assert_eq!(
        &patched[arrow.payload + 1..arrow.payload + 3],
        &[0xe7, 0x03],
        "stack size 999"
    );
    assert_eq!(patched.len(), body.len());
}

/// Dry run of JSON test mods in `PYLUNE_TEST_MODS=<dir>` against the
/// installed game's tables, in memory: every change must resolve and match.
#[test]
#[ignore = "reads the local game installation"]
fn test_mods_apply_to_the_real_tables() {
    let Ok(dir) = std::env::var("PYLUNE_TEST_MODS") else {
        return;
    };
    let group = game_dir().join("0008");
    let index = pamt::parse(&fs::read(group.join("0.pamt")).unwrap()).unwrap();
    let read = |p: &str| extract::extract(&group, p, &index.files[p]).unwrap();
    let files = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok());
    files
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .for_each(|e| {
            let parsed =
                crate::patch::parse::parse(&fs::read_to_string(e.path()).unwrap()).unwrap();
            parsed.patches.iter().for_each(|p| {
                let mut body = read(&p.game_file);
                let head = read(&p.game_file.replace("staticinfobody", "staticinfoheader"));
                let report = crate::patch::apply::apply(&mut body, p, Some(&head), true, false);
                println!(
                    "{}: {} applied, failed {:?}",
                    e.path().display(),
                    report.applied,
                    report.failed
                );
                assert!(report.failed.is_empty() && report.applied == p.changes.len());
            });
        });
}
