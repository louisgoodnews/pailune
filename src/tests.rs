//! End-to-end tests against a throw-away fake game directory.

use crate::backup::{MetaBackup, reset, snapshot};
use crate::config::Config;
use crate::mod_engine::model::{ModKind, Registry};
use crate::mod_engine::plan::{self, Placement};
use crate::mod_engine::{install, registry};
use crate::steam::{AsiLoader, GameStatus, locate};
use crate::symlink::{Manifest, PathState, deploy, path_state, undeploy};
use std::fs;
use std::path::{Path, PathBuf};

/// A fake installation plus staging folder inside a temp dir.
struct Fixture {
    _tmp: tempfile::TempDir,
    config: Config,
    staging: PathBuf,
    sources: PathBuf,
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("steamapps/common/Crimson Desert");
    write(&game.join("bin64/CrimsonDesert.exe"), "exe");
    write(&game.join("bin64/winmm.dll"), "loader");
    write(&game.join("meta/0.papgt"), "vanilla-papgt");
    write(&game.join("meta/0.paver"), "v1");
    write(&game.join("0000/0.paz"), "vanilla-archive");
    write(&game.join("0036/0.paz"), "foreign-overlay");
    fs::create_dir_all(tmp.path().join("steamapps/compatdata/3321460/pfx")).unwrap();
    let staging = tmp.path().join("staging");
    let sources = tmp.path().join("downloads");
    fs::create_dir_all(&staging).unwrap();
    // A zipped-style mod with a wrapper folder, an ASI and a readme.
    write(&sources.join("Cool Mod/Cool Mod v1/CoolMod.asi"), "asi");
    write(&sources.join("Cool Mod/Cool Mod v1/readme.txt"), "docs");
    write(&sources.join("Cool Mod/Cool Mod v1/patch.json"), "{}");
    // An overlay mod replacing the papgt and adding an archive folder.
    write(&sources.join("Overlay/meta/0.papgt"), "modded-papgt");
    write(&sources.join("Overlay/0037/0.paz"), "mod-archive");
    let config = Config {
        game_dir: game,
        ..Config::default()
    };
    Fixture {
        _tmp: tmp,
        config,
        staging,
        sources,
    }
}

fn enabled_registry(fx: &Fixture) -> Registry {
    let reg = registry::sync(&Registry::default(), &fx.staging).unwrap();
    (0..reg.mods.len()).fold(reg, |r, i| registry::toggled(&r, i))
}

#[test]
fn placement_rules() {
    assert_eq!(plan::placement(Path::new("x/Mod.ASI")), Placement::Bin64);
    assert_eq!(plan::placement(Path::new("0036/0.paz")), Placement::Mirror);
    assert_eq!(
        plan::placement(Path::new("bin64/foo.txt")),
        Placement::Mirror
    );
    assert_eq!(plan::placement(Path::new("cfg/a.json")), Placement::JsonDir);
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
    let game = &fx.config.game_dir;
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
    let report = deploy::deploy(&link_plan, &mut manifest).unwrap();
    assert_eq!(report.linked, 4);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(matches!(
        path_state(&game.join("bin64/CoolMod.asi")),
        PathState::Symlink(_)
    ));
    assert!(matches!(
        path_state(&game.join("bin64/mods/patch.json")),
        PathState::Symlink(_)
    ));
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
    let report = reset::clean_slate(&info, &mut manifest).unwrap();
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
    deploy::deploy(&link_plan, &mut manifest).unwrap();
    let papgt = fx.config.game_dir.join("meta/0.papgt");
    fs::remove_file(&papgt).unwrap();
    fs::write(&papgt, "steam-update").unwrap();
    let report = undeploy::undeploy(&mut manifest).unwrap();
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
    assert!(reset::clean_slate(&info, &mut Manifest::default()).is_err());
}
