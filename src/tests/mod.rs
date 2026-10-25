//! End-to-end tests against a throw-away fake game directory.

mod crash;
mod deployment;
mod jobs;
mod json_patches;
mod nexus_http;
mod overlay;
mod staging;
mod storage;
mod ui;

pub(super) use jobs::wait_all as wait_all_jobs;

use crate::config::Config;
use crate::mod_engine::model::Registry;
use crate::mod_engine::registry;
use std::fs;
use std::path::{Path, PathBuf};

/// A fake installation plus staging folder inside a temp dir.
pub(super) struct Fixture {
    _tmp: tempfile::TempDir,
    pub config: Config,
    pub staging: PathBuf,
    pub sources: PathBuf,
}

pub(super) fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

pub(super) fn fixture() -> Fixture {
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
    // A mod replacing the ASI loader and adding a plugin folder.
    write(&sources.join("Overlay/bin64/winmm.dll"), "modded-loader");
    write(
        &sources.join("Overlay/bin64/plugins/extra.asi"),
        "mod-plugin",
    );
    let mut config = Config::default();
    config.game.dir = game;
    Fixture {
        _tmp: tmp,
        config,
        staging,
        sources,
    }
}

pub(super) fn enabled_registry(fx: &Fixture) -> Registry {
    registry::with_all(
        &registry::sync(&Registry::default(), &fx.staging, 0).unwrap(),
        true,
    )
}
