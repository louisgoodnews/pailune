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

#[test]
fn the_guide_follows_progress_and_remembers_the_choice() {
    use crate::ui::guide::steps::Step;
    use crate::ui::guide::{GuideAction, GuidePage};
    use crate::ui::{DeployAction, Tab};
    let fx = fixture();
    let mut app = app(&fx);
    assert!(app.guide.open, "opens on first start");
    assert_eq!(app.guide_facts().current(), Some(Step::AddMod));
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Overlay"),
    ])));
    settle(&mut app);
    assert_eq!(app.guide_facts().current(), Some(Step::EnableMod));
    app.dispatch(Action::Mod(ModAction::SetAll(true)));
    app.dispatch(Action::Deploy(DeployAction::Apply));
    settle(&mut app);
    assert_eq!(app.guide_facts().current(), Some(Step::Play));
    // Closing an unfinished guide keeps it for the next start.
    app.dispatch(Action::Guide(GuideAction::Close));
    assert!(!app.guide.open && app.config.behavior.show_guide);
    app.dispatch(Action::Guide(GuideAction::Open(GuidePage::Topics)));
    assert_eq!((app.guide.open, app.guide.page), (true, GuidePage::Topics));
    app.dispatch(Action::Guide(GuideAction::ShowAtStart(false)));
    let saved = crate::config::file::load_or_create(&app.paths.config_file).unwrap();
    assert!(!saved.behavior.show_guide && !app.draft.config.behavior.show_guide);
    app.dispatch(Action::SwitchTab(Tab::Settings));
    assert!(!app.draft.is_changed(&app.config));
}

#[test]
fn mod_lists_rebuild_a_setup_and_place_mods_installed_later() {
    let fx = fixture();
    let mut app = app(&fx);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Cool Mod"),
        fx.sources.join("Overlay"),
    ])));
    settle(&mut app);
    // The setup to share: Overlay first and enabled, Cool Mod disabled.
    let first = app.registry.mods[1].name.clone();
    app.dispatch(Action::Mod(ModAction::Move(
        first,
        crate::mod_engine::model::MoveTo::Top,
    )));
    app.dispatch(Action::Mod(ModAction::SetEnabled("Overlay".into(), true)));
    let list = crate::modlist::capture(&app.registry, &[], "crimsondesert", 1);
    // Elsewhere only Cool Mod is installed, enabled.
    app.dispatch(Action::Mod(ModAction::Delete("Overlay".into())));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::SetAll(true)));
    let line = app.apply_list(list).unwrap();
    assert_eq!(line.level, Level::Warning, "{line:?}");
    assert!(!app.registry.mods[0].enabled, "Cool Mod disabled as listed");
    assert_eq!(app.mods.list_import.as_ref().unwrap().missing.len(), 1);
    // A change made after the import survives; installing the missing mod
    // puts only that mod where the list wants it.
    app.dispatch(Action::Mod(ModAction::SetEnabled("Cool Mod".into(), true)));
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Overlay"),
    ])));
    settle(&mut app);
    let order: Vec<_> = app
        .registry
        .mods
        .iter()
        .map(|m| (m.name.as_str(), m.enabled))
        .collect();
    assert_eq!(order, [("Overlay", true), ("Cool Mod", true)]);
    assert!(app.mods.list_import.is_none(), "nothing missing any more");
}

#[test]
fn fomod_installers_follow_the_chosen_options() {
    use crate::fomod::Selection;
    let fx = fixture();
    let mut app = app(&fx);
    let source = fx.sources.join("Better Things");
    crate::fomod::tests::write_mod(&source);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![source])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::Select("Better Things".into())));
    let installer = app.mods.details.as_ref().unwrap().1.installer.clone();
    assert!(installer.is_some(), "installer detected");
    let links = |app: &crate::ui::PyluneApp| {
        let entry = app
            .registry
            .mods
            .iter()
            .find(|m| m.name == "Better Things")
            .unwrap();
        let ops = crate::mod_engine::plan::mod_ops(
            &app.config,
            &app.paths.staging,
            &entry.name,
            entry.variant.as_deref(),
        )
        .unwrap();
        let mut names: Vec<String> = ops
            .iter()
            .map(|o| o.target.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    assert_eq!(links(&app), ["core.asi"], "defaults: no patch");
    let chosen = Selection {
        picks: vec![
            ("Textures".into(), "Resolution".into(), "4K".into()),
            ("Textures".into(), "Extras".into(), "Patch".into()),
        ],
        at: Vec::new(),
    };
    app.dispatch(Action::Mod(ModAction::SetVariant(
        "Better Things".into(),
        chosen.encode(),
    )));
    assert!(
        app.status.text.contains("installer options"),
        "{:?}",
        app.status
    );
    assert_eq!(links(&app), ["core.asi", "patch.asi", "patch.ini"]);
}
