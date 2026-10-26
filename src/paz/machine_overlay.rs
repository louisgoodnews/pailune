//! Read-only check of the overlay Pylune deployed into the local game
//! (`cargo test deployed_overlay -- --ignored --nocapture`): what it holds,
//! how it differs from the vanilla files, and where else the game keeps them.

use super::{extract, pamt, papgt};
use std::fs;

#[test]
#[ignore = "reads the local game installation"]
fn deployed_overlay_against_vanilla() {
    let game = crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID).unwrap();
    let table = papgt::parse(&fs::read(game.join("meta/0.papgt")).unwrap()).unwrap();
    let order: Vec<_> = table.groups.iter().map(|g| g.name.clone()).collect();
    println!("papgt order: {order:?}");
    let overlay = game.join("0100");
    let index = pamt::parse(&fs::read(overlay.join("0.pamt")).unwrap()).unwrap();
    for (path, record) in &index.files {
        println!("overlay: {path} {record:?}");
        let ours = extract::extract(&overlay, path, record).unwrap();
        let name = path.rsplit('/').next().unwrap();
        // Every vanilla group that holds a file with this name.
        for group in order.iter().filter(|g| *g != "0100") {
            let dir = game.join(group);
            let Ok(bytes) = fs::read(dir.join("0.pamt")) else {
                continue;
            };
            let Ok(vanilla) = pamt::parse(&bytes) else {
                continue;
            };
            for (vpath, vrec) in vanilla.files.iter().filter(|(p, _)| p.ends_with(name)) {
                let theirs = extract::extract(&dir, vpath, vrec).unwrap();
                let diffs: Vec<usize> = (0..ours.len().min(theirs.len()))
                    .filter(|i| ours[*i] != theirs[*i])
                    .collect();
                println!(
                    "  vanilla {group}/{vpath} {vrec:?}: len {} vs {}, {} differing bytes, first {:?}",
                    theirs.len(),
                    ours.len(),
                    diffs.len(),
                    diffs.iter().take(8).collect::<Vec<_>>()
                );
            }
        }
    }
}
