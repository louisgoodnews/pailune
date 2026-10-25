//! Staging: name races and case-insensitive game paths.

use super::{enabled_registry, fixture, write};
use crate::mod_engine::{install, plan};
use std::fs;

#[test]
fn mixed_case_paths_reuse_existing_game_folders() {
    let fx = fixture();
    write(&fx.sources.join("Shouty/BIN64/Loud.asi"), "x");
    write(&fx.sources.join("Shouty/BIN64/WINMM.DLL"), "y");
    install::import(&fx.sources.join("Shouty"), &fx.staging).unwrap();
    let link_plan = plan::build(&fx.config, &fx.staging, &enabled_registry(&fx)).unwrap();
    let targets: Vec<_> = link_plan.ops.iter().map(|op| op.target.clone()).collect();
    let game = &fx.config.game.dir;
    assert!(
        targets.contains(&game.join("bin64/Loud.asi")),
        "{targets:?}"
    );
    assert!(
        targets.contains(&game.join("bin64/winmm.dll")),
        "{targets:?}"
    );
}

#[test]
fn concurrent_same_name_imports_never_clobber() {
    let fx = fixture();
    let a = fx.sources.join("a/Same.asi");
    let b = fx.sources.join("b/Same.asi");
    write(&a, "first");
    write(&b, "second");
    let staging = fx.staging.clone();
    let results: Vec<_> = [a, b]
        .into_iter()
        .map(|src| {
            let staging = staging.clone();
            std::thread::spawn(move || install::import(&src, &staging))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    let content = fs::read_to_string(fx.staging.join("Same/Same.asi")).unwrap();
    assert!(content == "first" || content == "second");
    let leftovers = fs::read_dir(&fx.staging)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".pylune"))
        .count();
    assert_eq!(leftovers, 0, "no temp folders left behind");
}

#[test]
fn only_the_chosen_variant_is_deployed() {
    use crate::mod_engine::registry;
    let fx = fixture();
    write(
        &fx.sources.join("Creator/Human/0009/character/look.xml"),
        "human",
    );
    write(
        &fx.sources.join("Creator/Dwarf/0009/character/look.xml"),
        "dwarf",
    );
    write(&fx.sources.join("Creator/Creator.asi"), "shared");
    install::import(&fx.sources.join("Creator"), &fx.staging).unwrap();
    let reg = enabled_registry(&fx);
    let packed = |reg: &crate::mod_engine::model::Registry| {
        let input = crate::overlay::collect::collect(&fx.staging, reg).unwrap();
        fs::read_to_string(&input.packs["character/look.xml"]).unwrap()
    };
    assert_eq!(packed(&reg), "dwarf", "first option by default");
    let human = registry::with_variant(&reg, "Creator", Some("Human".into()));
    assert_eq!(packed(&human), "human");
    let links = plan::build(&fx.config, &fx.staging, &human).unwrap();
    assert!(
        links
            .ops
            .iter()
            .any(|op| op.target.ends_with("bin64/Creator.asi")),
        "shared files always deploy"
    );
    let details =
        crate::mod_engine::info::details(&fx.staging.join("Creator"), Some("Human")).unwrap();
    assert_eq!(details.variants, ["Dwarf", "Human"]);
    let dwarf_file = details
        .files
        .iter()
        .find(|f| f.rel.starts_with("Dwarf"))
        .unwrap();
    assert_eq!(
        dwarf_file.placement,
        plan::Placement::Skip,
        "other options are listed as not deployed"
    );

    let mut conn = crate::db::open(&fx.staging.with_file_name("v.sqlite3")).unwrap();
    crate::db::mods::update(&mut conn, |_| Ok(human.clone())).unwrap();
    assert_eq!(
        crate::db::mods::load(&conn).unwrap().mods[0]
            .variant
            .as_deref(),
        Some("Human")
    );
}
