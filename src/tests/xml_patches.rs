//! XML patch/merge and HTML patch files end to end: applied in load order on top of the
//! game's file, packed into the overlay, reverted by Unlink.

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

#[test]
fn xml_patches_and_merges_apply_in_load_order() {
    let fx = fixture();
    let game = fx.config.game.dir.clone();
    let xml = "ui/xml/texture/cd_challenges_00.xml";
    let vanilla = "\u{feff}<Texture Name=\"gauge\" Filename=\"UI/a.dds\" GetRect=\"73,105,14,18\"/>\r\n<Texture Name=\"icon\" Filename=\"UI/a.dds\" GetRect=\"1,1,70,70\"/>\r\n";
    let html = "ui/html/title.html";
    let page = "<div id=\"title\" class=\"big\"><img src=\"logo.png\"><span>Play</span></div>";
    let crc = write_group(
        &game.join("0012"),
        &[(xml, vanilla, 0x32), (html, page, 0x32)],
    );
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
            .join("A Wide/files/0012/ui/xml/texture/cd_challenges_00.xml.patch"),
        r#"{"operations": [{"op": "set-attr", "xpath": "//Texture[@Name='gauge']", "attribute": "GetRect", "value": "73,105,28,18"}]}"#,
    );
    write(
        &fx.sources
            .join("B Extra/0012/ui/xml/texture/cd_challenges_00.xml.merge"),
        "<xml-merge><Texture Name=\"gauge\" Filename=\"UI/b.dds\"/><Texture Name=\"new\" Filename=\"UI/n.dds\"/></xml-merge>",
    );

    write(
        &fx.sources
            .join("A Wide/files/0012/ui/html/title.html.patch"),
        "<html-patch><set at=\"#title\" class=\"+wide -big\"/><inner at=\"#title span\">Start</inner></html-patch>",
    );
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let pool = JobPool::spawn(1, paths.clone(), Arc::new(tasks::run), Arc::new(|| {}));
    let imports: Vec<_> = ["A Wide", "B Extra"]
        .iter()
        .map(|m| {
            let kind = JobKind::Import {
                source: fx.sources.join(m),
                replace: None,
            };
            pool.submit(kind, &fx.config)
        })
        .collect();
    assert!(wait_all_jobs(&pool, &imports).iter().all(Result::is_ok));
    let mut conn = db::open(&paths.database).unwrap();
    db::mods::update(&mut conn, |r| Ok(registry::with_all(r, true))).unwrap();
    let apply = pool.submit(JobKind::Apply, &fx.config);
    assert!(wait_all_jobs(&pool, &[apply]).remove(0).is_ok());

    let out = String::from_utf8(game_lookup_bytes(&game, xml).unwrap()).unwrap();
    assert_eq!(
        out,
        "\u{feff}<Texture Name=\"gauge\" Filename=\"UI/b.dds\" GetRect=\"73,105,28,18\"/>\r\n<Texture Name=\"icon\" Filename=\"UI/a.dds\" GetRect=\"1,1,70,70\"/>\r\n<Texture Name=\"new\" Filename=\"UI/n.dds\"/>\r\n"
    );
    let page_out = String::from_utf8(game_lookup_bytes(&game, html).unwrap()).unwrap();
    assert_eq!(
        page_out,
        "<div id=\"title\" class=\"wide\"><img src=\"logo.png\"><span>Start</span></div>"
    );
    let events = db::events::recent(&conn, 50, Some("A Wide")).unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.message.contains("XML patch applied")),
        "{events:?}"
    );

    let unlink = pool.submit(JobKind::UnlinkAll, &fx.config);
    assert!(wait_all_jobs(&pool, &[unlink])[0].is_ok());
    assert_eq!(game_lookup_bytes(&game, xml).unwrap(), vanilla.as_bytes());
}
