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
