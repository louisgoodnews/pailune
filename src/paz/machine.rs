//! Read-only checks against the locally installed game
//! (`cargo test -- --ignored`). Nothing is written.

use super::{Encryption, extract, pamt, papgt, paver};
use std::fs;
use std::path::PathBuf;

/// The installed game, found in the Steam libraries (tests on a real install only).
fn game_dir() -> PathBuf {
    crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID).expect("game installed")
}

#[test]
#[ignore = "reads the local game installation"]
fn parses_the_installed_archives() {
    let game = game_dir();
    let version = paver::parse(&fs::read(game.join("meta/0.paver")).unwrap()).unwrap();
    let table = papgt::parse(&fs::read(game.join("meta/0.papgt")).unwrap()).unwrap();
    println!("game {version}, {} groups", table.groups.len());
    let present: Vec<_> = table
        .groups
        .iter()
        .filter(|g| game.join(&g.name).join("0.pamt").is_file())
        .collect();
    let total: usize = present
        .iter()
        .map(|g| {
            let index = pamt::parse(&fs::read(game.join(&g.name).join("0.pamt")).unwrap()).unwrap();
            assert_eq!(
                index.crc, g.pamt_crc,
                "group {} registered with a different checksum",
                g.name
            );
            index.files.len()
        })
        .sum();
    println!("{} groups present, {total} files indexed", present.len());
    assert!(present.iter().all(|g| !g.is_optional) || present.len() > 30);

    // Decrypt and decompress one real encrypted XML.
    let (group, vpath, record) = present
        .iter()
        .find_map(|g| {
            let index = pamt::parse(&fs::read(game.join(&g.name).join("0.pamt")).unwrap()).unwrap();
            index
                .files
                .iter()
                .find(|(p, r)| {
                    p.ends_with(".xml")
                        && Encryption::from_flags(r.flags) == Encryption::ChaCha20
                        && r.orig_size < 200_000
                })
                .map(|(p, r)| (g.name.clone(), p.clone(), *r))
        })
        .expect("an encrypted xml exists");
    let data = extract::extract(&game.join(&group), &vpath, &record).unwrap();
    let text = String::from_utf8_lossy(&data[..data.len().min(80)]).into_owned();
    println!("{group}/{vpath}: {} bytes, starts {text:?}", data.len());
    assert!(
        text.trim_start_matches('\u{feff}')
            .trim_start()
            .starts_with('<'),
        "decrypted XML"
    );
}

/// Extracts `PYLUNE_EXTRACT=<vpath>` into `PYLUNE_OUT=<dir>` for inspection.
#[test]
#[ignore = "reads the local game installation"]
fn extract_for_inspection() {
    let (Ok(vpath), Ok(out)) = (std::env::var("PYLUNE_EXTRACT"), std::env::var("PYLUNE_OUT"))
    else {
        return;
    };
    let game = game_dir();
    let table = papgt::parse(&fs::read(game.join("meta/0.papgt")).unwrap()).unwrap();
    let found = table.groups.iter().find_map(|g| {
        let dir = game.join(&g.name);
        let index = pamt::parse(&fs::read(dir.join("0.pamt")).ok()?).ok()?;
        index.files.get(&vpath).map(|r| (dir.clone(), *r))
    });
    let (dir, record) = found.expect("file exists");
    let data = extract::extract(&dir, &vpath, &record).unwrap();
    let target = PathBuf::from(out).join(vpath.rsplit('/').next().unwrap());
    fs::write(&target, data).unwrap();
    println!("wrote {}", target.display());
}

/// Textures: PATHC round-trips byte for byte; for sampled vanilla DDS
/// entries the stored header matches the PATHC template and chunk sizes,
/// decoding restores the original size, and raw-chunk textures re-encode
/// to exactly the stored bytes.
#[test]
#[ignore = "reads the local game installation"]
fn textures_match_the_pathc() {
    use super::{Compression, dds, pathc};
    let game = game_dir();
    let raw_table = fs::read(game.join("meta/0.pathc")).unwrap();
    let table = pathc::parse(&raw_table).unwrap();
    assert_eq!(pathc::serialize(&table), raw_table, "PATHC round trip");
    let index = pamt::parse(&fs::read(game.join("0012/0.pamt")).unwrap()).unwrap();
    let mut textures: Vec<_> = index
        .files
        .iter()
        .filter(|(p, r)| {
            p.ends_with(".dds") && Compression::from_flags(r.flags) == Compression::DdsPartial
        })
        .collect();
    textures.sort_by(|a, b| a.0.cmp(b.0));
    let (mut checked, mut reencoded) = (0, 0);
    textures
        .iter()
        .step_by((textures.len() / 300).max(1))
        .for_each(|(vpath, record)| {
            let stored = extract::read_stored(&game.join("0012"), record).unwrap();
            let info = dds::info(&stored).unwrap();
            let (template, m) = table.lookup(vpath).expect("texture registered");
            assert_eq!(
                template,
                dds::template(&stored[..info.header_len]).as_slice(),
                "{vpath} template"
            );
            let stamped: Vec<u32> = (0..4)
                .map(|i| u32::from_le_bytes(stored[32 + i * 4..36 + i * 4].try_into().unwrap()))
                .collect();
            assert_eq!(m.to_vec(), stamped, "{vpath} chunk sizes");
            let decoded = dds::decode(&stored, record.orig_size as usize).unwrap();
            if dds::raw_chunk_sizes(&info) == m {
                let class = u32::from_le_bytes(stored[124..128].try_into().unwrap());
                assert_eq!(
                    dds::encode_raw(&decoded, class).unwrap().0,
                    stored,
                    "{vpath} re-encode"
                );
                reencoded += 1;
            }
            checked += 1;
        });
    println!("{checked} textures checked, {reencoded} raw-chunk textures re-encoded byte for byte");
    assert!(checked > 100 && reencoded > 10);
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
