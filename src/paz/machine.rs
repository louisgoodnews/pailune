//! Read-only checks against the locally installed game
//! (`cargo test -- --ignored`). Nothing is written.

use super::{Encryption, extract, pamt, papgt, paver};
use std::fs;
use std::path::PathBuf;

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
