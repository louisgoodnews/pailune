//! Read-only checks of the table reader against the installed game.

use super::table;
use crate::paz::{extract, pamt};
use std::fs;

#[test]
#[ignore = "reads the local game installation"]
fn reads_the_item_table() {
    let game = crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID)
        .expect("game installed");
    let group = game.join("0008");
    let index = pamt::parse(&fs::read(group.join("0.pamt")).unwrap()).unwrap();
    let base = "gamedata/binarystaticinfo__/bin/iteminfo";
    let read = |ext: &str| {
        let vpath = format!("{base}.{ext}");
        extract::extract(&group, &vpath, &index.files[&vpath]).unwrap()
    };
    let (header, body) = (read("staticinfoheader"), read("staticinfobody"));
    let table = table::parse_index(&header).expect("index tiles");
    let anchors = table.name_anchors(&body);
    println!(
        "key size {}, {} records, {} names, e.g. {:?}",
        table.key_size,
        table.offsets.len(),
        anchors.len(),
        anchors.keys().take(3).collect::<Vec<_>>()
    );
    assert_eq!((table.key_size, table.offsets.len()), (4, 6816));
    assert!(
        anchors.len() > 6000,
        "nearly every record has a readable name"
    );
    assert!(
        anchors
            .keys()
            .all(|n| !n.is_empty() && n.chars().all(|c| !c.is_control()))
    );
}
