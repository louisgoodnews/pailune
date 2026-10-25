//! Settings and backups through the UI: validation, saving, live
//! preferences and restoring a backup.

use super::fixture;
use super::ui::{app, settle};
use crate::ui::{Action, Confirm, Level, ModAction};

#[test]
fn settings_are_validated_saved_and_previewed() {
    use crate::ui::{SettingsPage, Tab};
    let fx = fixture();
    let mut app = app(&fx);
    app.draft.config.jobs.max_parallel = 99;
    app.dispatch(Action::SaveSettings);
    assert!(app.status.text.contains("parallel"), "{:?}", app.status);
    app.draft.reset(SettingsPage::Advanced);
    app.draft.config.ui.scale = 1.5;
    app.draft.config.mods.enable_new = true;
    assert!(app.draft.is_changed(&app.config));
    app.dispatch(Action::SaveSettings);
    assert!(!app.draft.is_changed(&app.config), "{:?}", app.status);
    let saved = crate::config::file::load_or_create(&app.paths.config_file).unwrap();
    assert_eq!((saved.ui.scale, saved.mods.enable_new), (1.5, true));
    // New mods now start enabled.
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Overlay"),
    ])));
    settle(&mut app);
    assert!(app.registry.mods.iter().all(|m| m.enabled));
    // Without confirmation, a delete goes straight to the trash.
    app.dispatch(Action::Ask(Confirm::DeleteMod("Overlay".into())));
    settle(&mut app);
    assert!(app.confirm.is_none() && app.registry.mods.is_empty());
    assert_eq!(app.tab, Tab::Mods);
}

#[test]
fn backups_restore_the_mod_list_and_settings() {
    use crate::ui::BackupAction;
    let fx = fixture();
    let mut app = app(&fx);
    settle(&mut app);
    assert_eq!(app.backups.len(), 1, "an automatic backup on first start");
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Overlay"),
    ])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::SetAll(true)));
    app.dispatch(Action::Backup(BackupAction::Create { full: true }));
    settle(&mut app);
    let backup = app
        .backups
        .iter()
        .find(|b| b.label == "manual")
        .expect("manual backup")
        .path
        .clone();
    app.dispatch(Action::Mod(ModAction::Delete("Overlay".into())));
    settle(&mut app);
    assert!(app.registry.mods.is_empty());
    app.dispatch(Action::Backup(BackupAction::Restore(backup)));
    settle(&mut app);
    assert_eq!(app.status.level, Level::Success, "{:?}", app.status);
    let names: Vec<_> = app
        .registry
        .mods
        .iter()
        .map(|m| (m.name.as_str(), m.enabled))
        .collect();
    assert_eq!(names, [("Overlay", true)]);
    assert!(
        app.backups.iter().any(|b| b.label == "before restore"),
        "safety backup taken"
    );
}
