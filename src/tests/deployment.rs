//! Staging, deployment, conflicts and reset.

use super::{enabled_registry, fixture, write};
use crate::backup::{MetaBackup, reset, snapshot};
use crate::mod_engine::conflicts;
use crate::mod_engine::model::ModKind;
use crate::mod_engine::plan::{self, Placement};
use crate::mod_engine::{install, registry};
use crate::steam::{AsiLoader, GameStatus, locate};
use crate::symlink::journal::NullJournal;
use crate::symlink::{Manifest, PathState, deploy, path_state, undeploy};
use std::fs;
use std::path::Path;

#[test]
fn placement_rules() {
    assert_eq!(plan::placement(Path::new("x/Mod.ASI")), Placement::Bin64);
    assert_eq!(plan::placement(Path::new("0036/0.paz")), Placement::Mirror);
    assert_eq!(
        plan::placement(Path::new("bin64/foo.txt")),
        Placement::Mirror
    );
    use crate::mod_engine::plan::PazFormat;
    let paz = |f| Placement::NeedsPaz(f);
    assert_eq!(
        plan::placement(Path::new("cfg/a.json")),
        paz(PazFormat::JsonPatch)
    );
    assert_eq!(
        plan::placement(Path::new("Mod/files/0012/ui/a.xml")),
        paz(PazFormat::Raw)
    );
    assert_eq!(
        plan::placement(Path::new("0012/ui/a.xml")),
        paz(PazFormat::Loose)
    );
    assert_eq!(plan::placement(Path::new("0036/0.paz")), Placement::Mirror);
    assert_eq!(
        plan::placement(Path::new("Mod/manifest.json")),
        Placement::Skip
    );
    assert_eq!(
        plan::placement(Path::new("tex/hair.dds")),
        paz(PazFormat::Loose)
    );
    assert_eq!(plan::placement(Path::new("readme.md")), Placement::Skip);
}

#[test]
fn locate_detects_game_prefix_and_loader() {
    let fx = fixture();
    let info = locate::resolve(&fx.config);
    assert_eq!(locate::game_status(&info), GameStatus::Ready);
    assert_eq!(locate::detect_loader(&info), AsiLoader::Winmm);
    assert!(info.prefix_dir.is_some());
}

#[test]
fn full_cycle_is_non_destructive() {
    let fx = fixture();
    let info = locate::resolve(&fx.config);
    let game = &fx.config.game.dir;
    assert_eq!(snapshot::ensure_meta_backup(&info).unwrap(), 2);
    assert_eq!(snapshot::meta_backup_state(&info), MetaBackup::Current);

    let a = install::import(&fx.sources.join("Cool Mod"), &fx.staging).unwrap();
    let b = install::import(&fx.sources.join("Overlay"), &fx.staging).unwrap();
    assert!(
        fx.staging.join(&a).join("CoolMod.asi").is_file(),
        "wrapper hoisted"
    );
    assert!(install::import(&fx.sources.join("Overlay"), &fx.staging).is_err());

    let reg = enabled_registry(&fx);
    let kinds: Vec<ModKind> = reg.mods.iter().map(|m| m.kind).collect();
    assert_eq!(kinds, vec![ModKind::Mixed, ModKind::Overlay]);
    assert_eq!(b, "Overlay");

    let link_plan = plan::build(&fx.config, &fx.staging, &reg).unwrap();
    let mut manifest = Manifest::default();
    let report = deploy::deploy(&link_plan, &fx.staging, &mut manifest, &mut NullJournal).unwrap();
    assert_eq!(
        report.linked, 3,
        "the JSON patch needs PAZ support and is not linked"
    );
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(matches!(
        path_state(&game.join("bin64/CoolMod.asi")),
        PathState::Symlink(_)
    ));
    assert!(!game.join("bin64/mods/patch.json").exists());
    assert_eq!(
        fs::read_to_string(game.join("meta/0.papgt")).unwrap(),
        "modded-papgt"
    );
    assert_eq!(
        fs::read_to_string(game.join("meta/0.papgt.bak")).unwrap(),
        "vanilla-papgt"
    );
    assert!(!game.join("bin64/readme.txt").exists());

    // Clean slate: links gone, originals back, foreign overlay archived.
    let report = reset::clean_slate(&info, &mut manifest, &mut NullJournal).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(
        fs::read_to_string(game.join("meta/0.papgt")).unwrap(),
        "vanilla-papgt"
    );
    assert!(!game.join("bin64/CoolMod.asi").exists());
    assert!(!game.join("0037").exists(), "created dirs removed");
    assert!(!game.join("bin64/mods").exists());
    assert!(game.join("0036.bak/0.paz").is_file());
    assert_eq!(snapshot::meta_backup_state(&info), MetaBackup::Current);
    assert_eq!(
        fs::read_to_string(game.join("0000/0.paz")).unwrap(),
        "vanilla-archive"
    );
    assert!(
        fx.staging.join("Overlay/meta/0.papgt").is_file(),
        "staging untouched"
    );
    assert_eq!(
        fs::read_to_string(fx.staging.join("Overlay/meta/0.papgt")).unwrap(),
        "modded-papgt"
    );

    assert_eq!(reset::restore_overlays(&info).unwrap(), 1);
    assert!(game.join("0036/0.paz").is_file());
}

#[test]
fn undeploy_leaves_foreign_replacements_alone() {
    let fx = fixture();
    install::import(&fx.sources.join("Overlay"), &fx.staging).unwrap();
    let reg = enabled_registry(&fx);
    let link_plan = plan::build(&fx.config, &fx.staging, &reg).unwrap();
    let mut manifest = Manifest::default();
    deploy::deploy(&link_plan, &fx.staging, &mut manifest, &mut NullJournal).unwrap();
    let papgt = fx.config.game.dir.join("meta/0.papgt");
    fs::remove_file(&papgt).unwrap();
    fs::write(&papgt, "steam-update").unwrap();
    let report = undeploy::undeploy(&mut manifest, &mut NullJournal).unwrap();
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(fs::read_to_string(&papgt).unwrap(), "steam-update");
}

#[test]
fn outdated_meta_backup_blocks_restore() {
    let fx = fixture();
    let info = locate::resolve(&fx.config);
    snapshot::ensure_meta_backup(&info).unwrap();
    fs::write(info.meta_dir.join("0.paver"), "v2").unwrap();
    assert_eq!(snapshot::meta_backup_state(&info), MetaBackup::Outdated);
    assert!(reset::clean_slate(&info, &mut Manifest::default(), &mut NullJournal).is_err());
}

#[test]
fn import_as_replaces_only_when_asked() {
    let fx = fixture();
    let v1 = fx.sources.join("v1.asi");
    let v2 = fx.sources.join("v2.asi");
    write(&v1, "one");
    write(&v2, "two");
    install::import_as(&v1, &fx.staging, "Camera", false).unwrap();
    assert!(install::import_as(&v2, &fx.staging, "Camera", false).is_err());
    install::import_as(&v2, &fx.staging, "Camera", true).unwrap();
    assert!(fx.staging.join("Camera/v2.asi").is_file());
    assert!(!fx.staging.join("Camera/v1.asi").exists());
    let archive = fx.sources.join("mod.7z");
    write(&archive, "7z");
    assert!(install::import(&archive, &fx.staging).is_err());
    assert!(
        !fx.staging.join("mod").exists(),
        "failed import leaves nothing behind"
    );
    assert_eq!(install::sanitize_name("../A: B?"), "_A_ B_");
}

#[test]
fn conflicts_report_winners_and_losers() {
    let fx = fixture();
    write(&fx.sources.join("A/bin64/shared.asi"), "a");
    write(&fx.sources.join("B/bin64/shared.asi"), "b");
    write(&fx.sources.join("B/only_b.asi"), "b");
    install::import(&fx.sources.join("A"), &fx.staging).unwrap();
    install::import(&fx.sources.join("B"), &fx.staging).unwrap();
    let reg = enabled_registry(&fx);
    let report = conflicts::analyze(&fx.config, &fx.staging, &reg).unwrap();
    let (a, b) = (&report["A"], &report["B"]);
    assert_eq!(
        (a.wins.len(), a.loses.len(), b.wins.len(), b.loses.len()),
        (0, 1, 1, 0)
    );
    assert_eq!(a.loses[0].file, Path::new("bin64/shared.asi"));
    assert_eq!(a.loses[0].other, "B");
    let only_a = registry::with_enabled(&reg, "B", false);
    assert!(
        conflicts::analyze(&fx.config, &fx.staging, &only_a)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn mixed_case_paths_reuse_existing_game_folders() {
    let fx = fixture();
    write(&fx.sources.join("Shouty/BIN64/Loud.asi"), "x");
    write(&fx.sources.join("Shouty/Meta/0.PAPGT"), "y");
    install::import(&fx.sources.join("Shouty"), &fx.staging).unwrap();
    let link_plan = plan::build(&fx.config, &fx.staging, &enabled_registry(&fx)).unwrap();
    let targets: Vec<_> = link_plan.ops.iter().map(|op| op.target.clone()).collect();
    let game = &fx.config.game.dir;
    assert!(
        targets.contains(&game.join("bin64/Loud.asi")),
        "{targets:?}"
    );
    assert!(targets.contains(&game.join("meta/0.papgt")), "{targets:?}");
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
