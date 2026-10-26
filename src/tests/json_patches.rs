//! Byte-patch JSON mods end to end: resolve, apply in load order, pack, revert.

use super::overlay::{game_lookup_bytes, write_group};
use super::{fixture, wait_all_jobs, write};
use crate::config::paths::DataPaths;
use crate::db;
use crate::jobs::pool::JobPool;
use crate::jobs::{JobKind, tasks};
use crate::mod_engine::registry;
use crate::paz::papgt::{self, GroupEntry, Papgt};
use std::fs;
use std::sync::Arc;

const BODY: &str = "gamedata/binarystaticinfo__/bin/iteminfo.staticinfobody";
const HEADER: &str = "gamedata/binarystaticinfo__/bin/iteminfo.staticinfoheader";

/// A two-record item table: "Potion" (stack 0x05) and "Arrow" (stack 0x10).
fn item_table() -> (Vec<u8>, Vec<u8>) {
    let record = |key: u32, name: &str, stack: u8| {
        [
            &key.to_le_bytes()[..],
            &(name.len() as u32).to_le_bytes(),
            name.as_bytes(),
            &[0, stack, 0, 0, 0],
        ]
        .concat()
    };
    let (a, b) = (record(1, "Potion", 0x05), record(2, "Arrow", 0x10));
    let header = [
        &2u16.to_le_bytes()[..],
        &1u32.to_le_bytes(),
        &0u32.to_le_bytes(),
        &2u32.to_le_bytes(),
        &(a.len() as u32).to_le_bytes(),
    ]
    .concat();
    ([a, b].concat(), header)
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|b| *b as char).collect()
}

#[test]
fn json_patches_apply_in_load_order_and_revert() {
    let fx = fixture();
    let game = fx.config.game.dir.clone();
    let (body, header) = item_table();
    let crc = write_group(
        &game.join("0008"),
        &[
            (BODY, &latin1(&body), 0x32),
            (HEADER, &latin1(&header), 0x30),
        ],
    );
    let table = Papgt {
        unknown0: 1,
        unknown1: 2,
        groups: vec![GroupEntry {
            is_optional: false,
            lang_mask: 0x7FFF,
            name: "0008".into(),
            pamt_crc: crc,
        }],
    };
    fs::write(game.join("meta/0.papgt"), papgt::serialize(&table).unwrap()).unwrap();
    // "Potion" (19-byte record): name ends at 14, stack at 15.
    // "Arrow" starts at 19: name ends at 32, stack at 33.
    write(
        &fx.sources.join("A Stacks/stacks.json"),
        r#"{ "name": "Stacks", "patches": [ { "game_file": "iteminfo.pabgb", "changes": [
        { "entry": "Potion", "rel_offset": 1, "original": "05", "patched": "63", "label": "potion stack" },
        { "offset": 33, "original": "10", "patched": "20", "label": "arrow stack" } ] } ] }"#,
    );
    write(
        &fx.sources.join("B Arrows/arrows.json"),
        r#"{ "name": "Arrows", "patches": [ { "game_file": "gamedata/binarystaticinfo__/bin/iteminfo.pabgb",
        "changes": [ { "entry": "Arrow", "rel_offset": 1, "original": "20", "patched": "ff" } ] } ] }"#,
    );
    write(
        &fx.sources.join("C Broken/broken.json"),
        r#"{ "patches": [ { "game_file": "iteminfo.pabgb",
        "changes": [ { "offset": 0, "original": "99", "patched": "00" } ] } ] }"#,
    );
    write(
        &fx.sources.join("D Fields/fields.json"),
        r#"{ "format": 3, "target": "iteminfo.pabgb", "intents": [{"entry": "Potion", "field": "_maxStackCount", "new": 99}] }"#,
    );

    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let pool = JobPool::spawn(1, paths.clone(), Arc::new(tasks::run), Arc::new(|| {}));
    let imports: Vec<_> = ["A Stacks", "B Arrows", "C Broken", "D Fields"]
        .iter()
        .map(|m| {
            pool.submit(
                JobKind::Import {
                    source: fx.sources.join(m),
                    replace: None,
                },
                &fx.config,
            )
        })
        .collect();
    assert!(wait_all_jobs(&pool, &imports).iter().all(Result::is_ok));
    let mut conn = db::open(&paths.database).unwrap();
    db::mods::update(&mut conn, |r| Ok(registry::with_all(r, true))).unwrap();
    let apply = pool.submit(JobKind::Apply, &fx.config);
    let message = wait_all_jobs(&pool, &[apply]).remove(0).unwrap();
    println!("{message:?}");

    let patched = game_lookup_bytes(&game, BODY).expect("patched table");
    assert_eq!(patched[15], 0x63, "entry anchor applied");
    assert_eq!(patched[33], 0xFF, "the later mod wins on the same byte");
    assert_eq!(patched.len(), body.len());
    // The overlay (first group) carries the table's header with its body.
    let first = papgt::parse(&fs::read(game.join("meta/0.papgt")).unwrap())
        .unwrap()
        .groups[0]
        .name
        .clone();
    let overlay =
        crate::paz::pamt::parse(&fs::read(game.join(first).join("0.pamt")).unwrap()).unwrap();
    assert!(overlay.files.contains_key(BODY) && overlay.files.contains_key(HEADER));
    let log = db::events::recent(&conn, 50, Some("C Broken")).unwrap();
    assert!(
        log.iter().any(|e| e.message.contains("do not match")),
        "{log:?}"
    );
    let fields = db::events::recent(&conn, 50, Some("D Fields")).unwrap();
    assert!(
        fields
            .iter()
            .any(|e| e.message.contains("no verified layout")),
        "{fields:?}"
    );

    let unlink = pool.submit(JobKind::UnlinkAll, &fx.config);
    assert!(wait_all_jobs(&pool, &[unlink])[0].is_ok());
    let restored = game_lookup_bytes(&game, BODY).unwrap();
    assert_eq!(restored, body);
}

#[test]
fn css_patches_merge_in_load_order() {
    let fx = fixture();
    let game = fx.config.game.dir.clone();
    let css = "ui/xml/gamemain/title/titleview.css";
    let vanilla = "#Copyright { color: #acacac; font-size: 15px; }\r\n.keep { x: 1 }\r\n";
    let crc = write_group(&game.join("0012"), &[(css, vanilla, 0x32)]);
    let table = Papgt {
        unknown0: 1,
        unknown1: 2,
        groups: vec![GroupEntry {
            is_optional: false,
            lang_mask: 0x7FFF,
            name: "0012".into(),
            pamt_crc: crc,
        }],
    };
    fs::write(game.join("meta/0.papgt"), papgt::serialize(&table).unwrap()).unwrap();
    write(
        &fx.sources
            .join("A Red/files/0012/ui/xml/gamemain/title/titleview.css.patch"),
        "#Copyright { color: #ff3030 }",
    );
    write(
        &fx.sources
            .join("B Big/0012/ui/xml/gamemain/title/titleview.css.merge"),
        "#Copyright { font-size: 30px }\n/* @add */\n.extra { y: 2 }",
    );

    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let pool = JobPool::spawn(1, paths.clone(), Arc::new(tasks::run), Arc::new(|| {}));
    let imports: Vec<_> = ["A Red", "B Big"]
        .iter()
        .map(|m| {
            pool.submit(
                JobKind::Import {
                    source: fx.sources.join(m),
                    replace: None,
                },
                &fx.config,
            )
        })
        .collect();
    assert!(wait_all_jobs(&pool, &imports).iter().all(Result::is_ok));
    let mut conn = db::open(&paths.database).unwrap();
    db::mods::update(&mut conn, |r| Ok(registry::with_all(r, true))).unwrap();
    let apply = pool.submit(JobKind::Apply, &fx.config);
    assert!(wait_all_jobs(&pool, &[apply]).remove(0).is_ok());

    let out = String::from_utf8(game_lookup_bytes(&game, css).unwrap()).unwrap();
    assert!(
        out.contains("#Copyright { color: #ff3030; font-size: 30px; }"),
        "{out}"
    );
    assert!(
        out.contains(".keep { x: 1 }\r\n") && out.ends_with(".extra { y: 2 }\r\n"),
        "{out:?}"
    );
}
