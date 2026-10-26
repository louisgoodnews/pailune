//! ASI plugins through the UI (D-12): who installed them, and editing their
//! settings, which reach the game through the deployed link.

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
    app.dispatch(Action::Mod(ModAction::SaveSettingsFile(
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
    app.dispatch(Action::Mod(ModAction::SaveSettingsFile(
        outside,
        String::new(),
    )));
    assert_eq!(
        app.status.level,
        Level::Error,
        "paths outside staging are refused"
    );
}
