//! UI actions end to end: a real `PyluneApp` with a headless egui context,
//! driven through `dispatch` and `poll` like the buttons do.

use super::{Fixture, fixture};
use crate::config::paths::DataPaths;
use crate::nexus::collection::{Collection, Member};
use crate::ui::{
    Action, Confirm, DeployAction, JobState, Level, ModAction, NexusAction, ProfileAction,
    PyluneApp,
};
use std::fs;
use std::time::{Duration, Instant};

pub(super) fn app(fx: &Fixture) -> PyluneApp {
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let conn = crate::db::open(&paths.database).unwrap();
    PyluneApp::build(
        eframe::egui::Context::default(),
        paths,
        fx.config.clone(),
        conn,
        None,
    )
}

/// Polls until no job is queued or running.
pub(super) fn settle(app: &mut PyluneApp) {
    let start = Instant::now();
    loop {
        app.poll();
        let busy = app
            .jobs
            .iter()
            .any(|j| matches!(j.state, JobState::Queued | JobState::Running));
        if !busy || start.elapsed() > Duration::from_secs(20) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Runs the next poll as if a second had passed (process scan, spool).
fn tick(app: &mut PyluneApp) {
    app.last_poll = Instant::now() - Duration::from_secs(2);
    app.poll();
}

#[test]
fn import_enable_apply_and_delete() {
    let fx = fixture();
    let mut app = app(&fx);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Cool Mod"),
        fx.sources.join("Overlay"),
    ])));
    settle(&mut app);
    assert_eq!(app.registry.mods.len(), 2);
    assert!(!app.dirty, "nothing enabled yet");
    app.dispatch(Action::Mod(ModAction::SetAll(true)));
    assert!(app.dirty);
    app.dispatch(Action::Deploy(DeployAction::Apply));
    settle(&mut app);
    assert!(!app.dirty, "{:?}", app.status);
    assert_eq!(app.deployed_links, 3);
    app.dispatch(Action::Mod(ModAction::Select("Cool Mod".into())));
    assert!(
        app.mods
            .details
            .as_ref()
            .is_some_and(|(n, d)| n == "Cool Mod" && !d.readmes.is_empty())
    );
    app.config.behavior.confirm_deletes = true;
    app.dispatch(Action::Ask(Confirm::DeleteMod("Cool Mod".into())));
    assert!(app.confirm.is_some(), "asks when the user wants it");
    app.dispatch(Action::Mod(ModAction::Delete("Cool Mod".into())));
    settle(&mut app);
    assert_eq!(
        (
            app.registry.mods.len(),
            app.deployed_links,
            app.confirm.clone()
        ),
        (1, 2, None)
    );
    assert!(
        app.mods.selected.is_none(),
        "selection cleared with the mod"
    );
    // Deletes are recoverable: Undo brings the mod back with its files.
    let undo = app.mods.undo.clone().expect("undo offered");
    assert_eq!(app.mods.trash.len(), 1);
    app.dispatch(Action::Mod(ModAction::RestoreDeleted(undo)));
    settle(&mut app);
    assert!(
        app.registry.mods.iter().any(|m| m.name == "Cool Mod"),
        "{:?}",
        app.status
    );
    assert!(fx.staging.join("Cool Mod/CoolMod.asi").is_file());
    assert!(app.mods.trash.is_empty() && app.mods.undo.is_none());
}

#[test]
fn profiles_save_and_load() {
    let fx = fixture();
    let mut app = app(&fx);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Cool Mod"),
        fx.sources.join("Overlay"),
    ])));
    settle(&mut app);
    app.dispatch(Action::Mod(ModAction::SetEnabled("Overlay".into(), true)));
    app.dispatch(Action::Profile(ProfileAction::Save("Loader only".into())));
    app.dispatch(Action::Mod(ModAction::SetAll(true)));
    app.dispatch(Action::Profile(ProfileAction::Load("Loader only".into())));
    let enabled: Vec<_> = app
        .registry
        .mods
        .iter()
        .filter(|m| m.enabled)
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(enabled, ["Overlay"]);
    assert_eq!(app.profiles.book.active.as_deref(), Some("Loader only"));
    app.dispatch(Action::Profile(ProfileAction::Save("  ".into())));
    assert_eq!(app.status.level, Level::Error, "empty names are refused");
}

#[test]
fn nxm_links_need_a_key_and_collection_members_skip_the_dialog() {
    let fx = fixture();
    let mut app = app(&fx);
    let spool =
        |name: &str, url: &str| fs::write(app_spool(&fx).join(format!("{name}.req")), url).unwrap();
    spool("1", "nxm://crimsondesert/mods/7/files/70");
    tick(&mut app);
    assert!(
        app.nexus.incoming.is_empty() && app.status.level == Level::Error,
        "keyless link ignored"
    );
    spool(
        "2",
        "nxm://crimsondesert/mods/7/files/70?key=abc&expires=9999999999",
    );
    tick(&mut app);
    assert_eq!(app.nexus.incoming.len(), 1, "waits for confirmation");
    app.dispatch(Action::Nexus(NexusAction::RejectIncoming(0)));
    assert!(app.nexus.incoming.is_empty());

    // A free account installs a collection: nothing downloads, members wait.
    let member = Member {
        position: 0,
        mod_id: 8,
        file_id: 80,
        optional: false,
        selected: true,
        title: "M".into(),
        size: 1,
        mod_name: None,
    };
    app.nexus.preview = Some(Collection {
        slug: "abc".into(),
        name: "Test".into(),
        revision: 1,
        members: vec![member],
    });
    app.dispatch(Action::Nexus(NexusAction::InstallCollection));
    assert_eq!(app.status.level, Level::Warning, "{:?}", app.status);
    assert_eq!(app.nexus.collections.len(), 1);
    spool(
        "3",
        "nxm://crimsondesert/mods/8/files/80?key=abc&expires=9999999999",
    );
    tick(&mut app);
    assert!(
        app.nexus.incoming.is_empty(),
        "the member was chosen already"
    );
    let download = app.jobs.iter().any(|j| matches!(&j.kind, crate::jobs::JobKind::Download(d) if d.collection.as_deref() == Some("abc")));
    assert!(download);
    settle(&mut app);
    app.dispatch(Action::Nexus(NexusAction::UninstallCollection(
        "abc".into(),
    )));
    assert!(app.nexus.collections.is_empty());
}

fn app_spool(fx: &Fixture) -> std::path::PathBuf {
    let spool = fx.staging.parent().unwrap().join("spool");
    fs::create_dir_all(&spool).unwrap();
    spool
}
