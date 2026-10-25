//! Dry run against the installed game: builds the overlay into a scratch
//! folder from the live (vanilla) metadata. Reads the game, writes nothing
//! there. `PYLUNE_TEST_MOD=<raw mod folder> PYLUNE_OUT=<dir> cargo test -- --ignored`.

use super::{OverlayInput, build};
use crate::paz::{extract, pamt, papgt};
use std::fs;
use std::path::PathBuf;

#[test]
#[ignore = "reads the local game installation"]
fn dry_run_against_the_installed_game() {
    let (Ok(module), Ok(out)) = (
        std::env::var("PYLUNE_TEST_MOD"),
        std::env::var("PYLUNE_OUT"),
    ) else {
        return;
    };
    let mut config = crate::config::Config::default();
    config.game.dir = crate::steam::library::find_game(config.game.app_id).expect("game installed");
    let info = crate::steam::locate::resolve(&config);
    let root = PathBuf::from(module);
    let files = crate::mod_engine::plan::deployable_files(&root).unwrap();
    let packs = files
        .iter()
        .filter_map(|(rel, _)| {
            crate::mod_engine::plan::virtual_path(rel).map(|v| (v, root.join(rel)))
        })
        .collect();
    let input = OverlayInput {
        packs,
        ..OverlayInput::default()
    };
    let vanilla = fs::read(info.meta_dir.join("0.papgt")).unwrap();
    let out = PathBuf::from(out);
    let plan = build::build_from(&info, &vanilla, &input, &out).unwrap();
    println!("{plan:#?}");
    let table = papgt::parse(&fs::read(out.join("meta/0.papgt")).unwrap()).unwrap();
    assert_eq!(
        table.groups.len(),
        papgt::parse(&vanilla).unwrap().groups.len() + 1
    );
    // Resolve every packed path like the game: our group first, then vanilla.
    input.packs.iter().for_each(|(vpath, source)| {
        let dir = table
            .groups
            .iter()
            .map(|g| match out.join(&g.name).is_dir() {
                true => out.join(&g.name),
                false => info.game_dir.join(&g.name),
            });
        let (group, record) = dir
            .filter_map(|d| {
                let index = pamt::parse(&fs::read(d.join("0.pamt")).ok()?).ok()?;
                index.files.get(vpath).map(|r| (d.clone(), *r))
            })
            .next()
            .expect("resolvable");
        assert!(group.starts_with(&out), "{vpath} resolves to the overlay");
        assert_eq!(
            extract::extract(&group, vpath, &record).unwrap(),
            fs::read(source).unwrap()
        );
        println!(
            "{vpath}: flags {:#06x}, served from {}",
            record.flags,
            group.display()
        );
    });
}
