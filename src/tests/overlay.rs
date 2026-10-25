//! The archive overlay end to end: build, deploy, game-style lookup, revert.

use super::{fixture, wait_all_jobs, write};
use crate::config::paths::DataPaths;
use crate::db;
use crate::jobs::pool::JobPool;
use crate::jobs::{JobKind, tasks};
use crate::mod_engine::registry;
use crate::paz::papgt::{self, GroupEntry, Papgt};
use crate::paz::pathc::{self, Pathc};
use crate::paz::write::{self as group_write, Packed};
use crate::paz::{encode, extract, pamt};
use crate::symlink::{PathState, path_state};
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Writes a pack group with the given (vpath, content, flags) files.
fn write_group(dir: &Path, files: &[(&str, &str, u16)]) -> u32 {
    let packed = files
        .iter()
        .map(|(vpath, data, flags)| Packed {
            vpath: vpath.to_string(),
            stored: encode::encode(vpath, data.as_bytes(), *flags).unwrap(),
            orig_size: data.len() as u32,
            flags: *flags,
        })
        .collect();
    let group = group_write::build(packed);
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("0.paz"), &group.paz).unwrap();
    fs::write(dir.join("0.pamt"), &group.pamt).unwrap();
    group.pamt_crc
}

/// A 16×8 DXT5 texture with 3 mips whose pixel bytes are all `fill`.
fn dds(fill: u8) -> Vec<u8> {
    let mut file = vec![0u8; 128];
    file[0..4].copy_from_slice(b"DDS ");
    file[4..8].copy_from_slice(&124u32.to_le_bytes());
    file[12..16].copy_from_slice(&8u32.to_le_bytes());
    file[16..20].copy_from_slice(&16u32.to_le_bytes());
    file[28..32].copy_from_slice(&3u32.to_le_bytes());
    file[84..88].copy_from_slice(b"DXT5");
    file.extend(vec![fill; 128 + 32 + 16]);
    file
}

/// Looks a file up the way the game does: first registered group wins.
fn game_lookup(game: &Path, vpath: &str) -> Option<String> {
    let table = papgt::parse(&fs::read(game.join("meta/0.papgt")).ok()?).ok()?;
    table.groups.iter().find_map(|g| {
        let dir = game.join(&g.name);
        let index = pamt::parse(&fs::read(dir.join("0.pamt")).ok()?).ok()?;
        let record = index.files.get(vpath)?;
        let data = extract::extract(&dir, vpath, record).ok()?;
        Some(String::from_utf8_lossy(&data).into_owned())
    })
}

#[test]
fn overlay_overrides_game_files_and_reverts_exactly() {
    let fx = fixture();
    let game = fx.config.game.dir.clone();
    let vanilla_crc = write_group(
        &game.join("0000"),
        &[
            ("ui/title.html", "<html>vanilla</html>", 0x32),
            ("ui/menu.css", "a{color:red}", 0x02),
        ],
    );
    let table = Papgt {
        unknown0: 0x12FA,
        unknown1: 2,
        groups: vec![
            GroupEntry {
                is_optional: false,
                lang_mask: 0x7FFF,
                name: "0000".into(),
                pamt_crc: vanilla_crc,
            },
            GroupEntry {
                is_optional: true,
                lang_mask: 0x0080,
                name: "0036".into(),
                pamt_crc: 0,
            },
        ],
    };
    let vanilla_papgt = papgt::serialize(&table).unwrap();
    fs::write(game.join("meta/0.papgt"), &vanilla_papgt).unwrap();
    // A vanilla texture table with one registered texture.
    let mut textures = Pathc::empty();
    crate::overlay::textures::pack("ui/icon.dds", &dds(0xAA), 0x0001, &mut textures).unwrap();
    let vanilla_pathc = pathc::serialize(&textures);
    fs::write(game.join("meta/0.pathc"), &vanilla_pathc).unwrap();
    fs::remove_dir_all(game.join("0036")).unwrap();
    let vanilla_paz = fs::read(game.join("0000/0.paz")).unwrap();

    write(
        &fx.sources.join("Radial/Radial/files/0000/ui/title.html"),
        "<html>modded</html>",
    );
    let icon_path = fx.sources.join("Radial/Radial/files/0000/ui/icon.dds");
    fs::create_dir_all(icon_path.parent().unwrap()).unwrap();
    fs::write(&icon_path, dds(0x55)).unwrap();
    write(
        &fx.sources.join("Radial/Radial/files/0000/ui/broken.dds"),
        "not a texture",
    );
    write(&fx.sources.join("Radial/Radial/manifest.json"), "{}");
    write_group(
        &fx.sources.join("Prebuilt/0042"),
        &[("ui/new.xml", "<new/>", 0x00)],
    );

    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let pool = JobPool::spawn(2, paths.clone(), Arc::new(tasks::run), Arc::new(|| {}));
    let imports: Vec<_> = ["Radial", "Prebuilt"]
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

    let table = papgt::parse(&fs::read(game.join("meta/0.papgt")).unwrap()).unwrap();
    let names: Vec<_> = table.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(
        names,
        ["0100", "0101", "0000", "0036"],
        "overlays registered first"
    );
    assert!(matches!(
        path_state(&game.join("0100")),
        PathState::Symlink(_)
    ));
    assert_eq!(
        game_lookup(&game, "ui/title.html").as_deref(),
        Some("<html>modded</html>")
    );
    assert_eq!(
        game_lookup(&game, "ui/menu.css").as_deref(),
        Some("a{color:red}")
    );
    assert_eq!(game_lookup(&game, "ui/new.xml").as_deref(), Some("<new/>"));
    let icon = game_lookup(&game, "ui/icon.dds").expect("texture packed");
    assert!(
        icon.as_bytes()[128..].iter().all(|b| *b == 0x55),
        "modded texture served"
    );
    let live_pathc = pathc::parse(&fs::read(game.join("meta/0.pathc")).unwrap()).unwrap();
    assert_eq!(
        live_pathc.lookup("ui/icon.dds").map(|(_, m)| m),
        Some([128, 32, 16, 0])
    );
    assert_eq!(
        game_lookup(&game, "ui/broken.dds"),
        None,
        "invalid textures are skipped"
    );
    assert_eq!(
        fs::read(game.join("0000/0.paz")).unwrap(),
        vanilla_paz,
        "vanilla archives untouched"
    );

    let unlink = pool.submit(JobKind::UnlinkAll, &fx.config);
    assert!(wait_all_jobs(&pool, &[unlink])[0].is_ok());
    assert_eq!(fs::read(game.join("meta/0.papgt")).unwrap(), vanilla_papgt);
    assert_eq!(fs::read(game.join("meta/0.pathc")).unwrap(), vanilla_pathc);
    assert!(!game.join("0100").exists() && !game.join("0101").exists());
    assert_eq!(
        game_lookup(&game, "ui/title.html").as_deref(),
        Some("<html>vanilla</html>")
    );
}
