//! ASI plugins through the UI (D-12): who installed them, and editing their
//! settings, which reach the game through the deployed link; JSON patches
//! edited in the details panel.

use super::fixture;
use super::ui::{app, settle};
use crate::ui::{Action, DeployAction, Level, ModAction};
use std::fs;

#[test]
fn plugins_are_listed_and_their_settings_edited() {
    let fx = fixture();
    let mut app = app(&fx);
    let source = fx.sources.join("Skipper");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("skipper.asi"), "asi").unwrap();
    fs::write(source.join("skipper.ini"), "[Main]\nSpeed=1\n").unwrap();
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![source])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::SetEnabled("Skipper".into(), true)));
    app.dispatch(Action::Deploy(DeployAction::Apply));
    settle(&mut app);
    let plugin = app
        .plugins
        .iter()
        .find(|p| p.name == "skipper.asi")
        .expect("listed");
    assert_eq!(plugin.owner.as_deref(), Some("Skipper"));
    assert!(plugin.ini.is_some());

    let staged = app.paths.staging.join("Skipper/skipper.ini");
    app.dispatch(Action::Mod(ModAction::SaveModFile(
        staged,
        "[Main]\nSpeed=3\n".into(),
    )));
    assert_eq!(app.status.level, Level::Success, "{:?}", app.status);
    let deployed = app.info.bin64_dir.join("skipper.ini");
    assert_eq!(
        fs::read_to_string(deployed).unwrap(),
        "[Main]\nSpeed=3\n",
        "seen through the link"
    );

    let outside = app.paths.staging.join("../escape.ini");
    app.dispatch(Action::Mod(ModAction::SaveModFile(outside, String::new())));
    assert_eq!(
        app.status.level,
        Level::Error,
        "paths outside staging are refused"
    );
}

#[test]
fn saving_settings_keeps_hard_links_recognised() {
    use crate::symlink::{journal::Journal, stamp};
    let fx = fixture();
    let mut app = app(&fx);
    let source = fx.sources.join("Skipper");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("skipper.asi"), "asi").unwrap();
    fs::write(source.join("skipper.ini"), "[Main]\nSpeed=1\n").unwrap();
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![source])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::SetEnabled("Skipper".into(), true)));
    app.dispatch(Action::Deploy(DeployAction::Apply));
    settle(&mut app);
    // Turn the deployed symlink into a hard link, as on Windows (D-11).
    let staged = app.paths.staging.join("Skipper/skipper.ini");
    let deployed = app.info.bin64_dir.join("skipper.ini");
    fs::remove_file(&deployed).unwrap();
    fs::hard_link(&staged, &deployed).unwrap();
    let mut journal = crate::db::links::DbJournal { conn: &app.conn };
    journal
        .link_stamped(&deployed, &stamp(&deployed).unwrap())
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(20));
    app.dispatch(Action::Mod(ModAction::SaveModFile(
        staged,
        "[Main]\nSpeed=42\n".into(),
    )));
    let record = crate::db::links::load(&app.conn)
        .unwrap()
        .links
        .into_iter()
        .find(|r| r.target == deployed)
        .expect("journaled");
    assert_eq!(record.stamp, stamp(&deployed), "stamp follows the rewrite");
}

#[test]
fn edited_json_patches_need_an_apply() {
    use crate::patch::document::{Slot, check, set};
    let fx = fixture();
    let mut app = app(&fx);
    let source = fx.sources.join("Stacks");
    fs::create_dir_all(&source).unwrap();
    let json = r#"{"patches": [{"game_file": "gamedata/iteminfo.pabgb", "changes": [
        {"entry": "Arrow", "rel_offset": 1, "original": "64000000", "patched": "e7030000"}]}]}"#;
    fs::write(source.join("stacks.json"), json).unwrap();
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![source])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::SetEnabled("Stacks".into(), true)));
    // As after an Apply (the fixture has no real archives to patch).
    let (_, revision) = crate::db::mods::load_with_revision(&app.conn).unwrap();
    crate::db::mods::set_deployed_revision(&app.conn, Some(revision)).unwrap();
    app.refresh_environment();
    assert!(!app.dirty);

    let slot = Slot::Patched {
        patch: 0,
        change: 0,
    };
    let edited = set(json, slot, serde_json::json!("f4010000")).unwrap();
    let staged = app.paths.staging.join("Stacks/stacks.json");
    app.dispatch(Action::Mod(ModAction::SaveModFile(staged.clone(), edited)));
    assert!(app.status.text.contains("press Apply"), "{:?}", app.status);
    assert!(app.dirty, "the banner asks for Apply");
    let saved = check(&fs::read_to_string(staged).unwrap()).unwrap();
    assert_eq!(
        saved.patches[0].changes[0].edit,
        crate::patch::Edit::Replace {
            original: Some(vec![0x64, 0, 0, 0]),
            patched: vec![0xf4, 0x01, 0, 0]
        }
    );
}
