//! UI actions end to end: a real `PailuneApp` with a headless egui context,
//! driven through `dispatch` and `poll` like the buttons do.

use super::{Fixture, fixture};
use crate::config::paths::DataPaths;
use crate::nexus::collection::{Collection, Member};
use crate::ui::{
    Action, Confirm, DeployAction, JobState, Level, ModAction, NexusAction, PailuneApp,
    ProfileAction,
};
use std::fs;
use std::time::{Duration, Instant};

pub(super) fn app(fx: &Fixture) -> PailuneApp {
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    let conn = crate::db::open(&paths.database).unwrap();
    PailuneApp::build(
        eframe::egui::Context::default(),
        paths,
        fx.config.clone(),
        conn,
        None,
    )
}

/// Polls until no job is queued or running.
pub(super) fn settle(app: &mut PailuneApp) {
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
fn tick(app: &mut PailuneApp) {
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

#[test]
fn mods_move_to_a_load_order_number() {
    use crate::mod_engine::model::MoveTo;
    let fx = fixture();
    let mut app = app(&fx);
    app.dispatch(Action::Mod(ModAction::ImportPaths(vec![
        fx.sources.join("Cool Mod"),
        fx.sources.join("Overlay"),
    ])));
    settle(&mut app);
    let first = app.registry.mods[0].name.clone();
    app.dispatch(Action::Mod(ModAction::Move(
        first.clone(),
        MoveTo::Position(2),
    )));
    assert_eq!(app.registry.mods[1].name, first);
}

#[test]
fn creates_new_json_mod_with_metadata_and_opens_it() {
    let fx = fixture();
    let mut app = app(&fx);

    // 1. Open prompt
    app.dispatch(Action::Mod(ModAction::CreateModPrompt));
    assert!(app.mods.create_mod.is_some());

    // 2. Submit new mod creation
    app.dispatch(Action::Mod(ModAction::CreateMod {
        name: "Carry Capacity 500".into(),
        author: "Khione".into(),
        version: "1.2".into(),
        description: "Increase max carry weight".into(),
    }));

    // 3. Prompt closes, mod is registered and selected
    assert!(app.mods.create_mod.is_none());
    assert_eq!(app.mods.selected.as_deref(), Some("Carry Capacity 500"));
    assert!(app.registry.mods.iter().any(|m| m.name == "Carry Capacity 500"));

    // 4. Staged file is valid format 3 JSON patch
    let staged_file = fx.staging.join("Carry Capacity 500/Carry Capacity 500.field.json");
    assert!(staged_file.is_file());
    let text = fs::read_to_string(&staged_file).unwrap();
    let json = crate::patch::document::check(&text).expect("valid json mod");
    assert_eq!(json.title, "Carry Capacity 500");
}

#[test]
fn data_browser_tab_and_record_inspection() {
    use crate::ui::DataBrowserAction;

    let fx = fixture();
    let mut app = app(&fx);

    // 1. Switch to Data Browser tab
    app.dispatch(Action::SwitchTab(crate::ui::Tab::DataBrowser));
    assert_eq!(app.tab, crate::ui::Tab::DataBrowser);

    // 2. Mock a loaded table and records in browser state
    let key = 0x0601u16;
    let price = 1500u64;
    let mut body = Vec::new();
    body.extend_from_slice(&key.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    let payload = body.len();
    body.push(0); // _isBlocked (false)
    body.extend_from_slice(&price.to_le_bytes()); // _increasePrice
    body.push(1);
    let end = body.len();

    app.data_browser.selected_table = Some("wantedinfo".into());
    app.data_browser.records = vec![crate::tables::Record {
        start: 0,
        key: key as u64,
        name: "target_01".into(),
        payload,
        end,
    }];
    app.data_browser.body = body;

    // 3. Select record
    app.dispatch(Action::DataBrowser(DataBrowserAction::SelectRecord(key as u64)));
    assert_eq!(app.data_browser.selected_record, Some(key as u64));

    let inspected = app.data_browser.inspected.as_ref().expect("inspected record");
    assert_eq!(inspected.name, "target_01");
    assert_eq!(inspected.key, key as u64);
    assert_eq!(inspected.fields.len(), 3);
    assert_eq!(inspected.fields[0].name, "_isBlocked");
    assert_eq!(inspected.fields[0].value, "false");
    assert_eq!(inspected.fields[1].name, "_increasePrice");
    assert_eq!(inspected.fields[1].value, "1500");
}

#[test]
fn inject_change_from_browser_into_mod() {
    use crate::ui::DataBrowserAction;

    let fx = fixture();
    let mut app = app(&fx);

    // 1. Start an edit on a field
    let edit = crate::ui::ActiveFieldEdit {
        field_name: "_increasePrice".into(),
        field_type: "u64".into(),
        original_value: "1500".into(),
        new_value: "9999".into(),
        is_raw: false,
        rel_offset: 1,
        original_hex: "dc05000000000000".into(),
    };
    app.dispatch(Action::DataBrowser(DataBrowserAction::StartFieldEdit(edit)));
    assert!(app.data_browser.active_edit.is_some());

    // 2. Dispatch AddChangeToMod
    app.dispatch(Action::DataBrowser(DataBrowserAction::AddChangeToMod {
        mod_name: "BountyBoost".into(),
        table_stem: "wantedinfo".into(),
        record_name: "target_01".into(),
        record_key: 0x0601,
        field_name: Some("_increasePrice".into()),
        new_value: "9999".into(),
        is_raw: false,
        rel_offset: 1,
        original_hex: "dc05000000000000".into(),
    }));

    // 3. Edit should be closed and mod selected
    assert!(app.data_browser.active_edit.is_none());
    assert_eq!(app.mods.selected.as_deref(), Some("BountyBoost"));

    // 4. Staging file should contain the injected format-3 intent
    let file = fx.staging.join("BountyBoost").join("BountyBoost.field.json");
    assert!(file.exists());
    let text = std::fs::read_to_string(&file).expect("read file");
    let json = crate::patch::document::check(&text).expect("valid format 3 mod");
    assert_eq!(json.patches.len(), 1);
    assert_eq!(json.patches[0].fields.len(), 1);
    let intent = &json.patches[0].fields[0];
    assert_eq!(intent.field.as_deref(), Some("_increasePrice"));
    assert_eq!(intent.new, serde_json::json!(9999));
    assert_eq!(intent.entry.as_deref(), Some("target_01"));
    assert_eq!(intent.key, Some(0x0601));
}

#[test]
fn dry_run_verifies_intents_and_detects_errors() {
    use crate::patch::dry_run::{DryRunStatus, check_patch_data};
    use crate::patch::FilePatch;
    use crate::tables::format3::Intent;

    let key = 0x0601u16;
    let price = 1500u64;

    // Body: key (u16) | u32 name_len | name | payload
    let name = "target_01";
    let mut payload = Vec::new();
    payload.push(0); // _isBlocked (false)
    payload.extend_from_slice(&price.to_le_bytes()); // _increasePrice
    payload.push(1);

    let body = [
        &key.to_le_bytes()[..],
        &(name.len() as u32).to_le_bytes(),
        name.as_bytes(),
        &payload,
    ]
    .concat();

    // WantedInfo header index: 1 record, key_size = 2, offset = 0
    let header = [
        &1u16.to_le_bytes()[..],
        &key.to_le_bytes(),
        &0u32.to_le_bytes(),
    ]
    .concat();

    let valid_patch = FilePatch {
        game_file: "gamedata/binarystaticinfo__/bin/wantedinfo.staticinfobody".into(),
        signature: None,
        changes: Vec::new(),
        fields: vec![Intent {
            entry: Some("target_01".into()),
            key: Some(key as u64),
            field: Some("_increasePrice".into()),
            op: "set".into(),
            new: serde_json::json!(9999),
            old: None,
            matches: None,
        }],
    };

    let res = check_patch_data(&valid_patch, Some(&header), &body, true);
    assert_eq!(res.intents.len(), 1);
    assert_eq!(res.intents[0], DryRunStatus::Verified);

    let invalid_patch = FilePatch {
        game_file: "gamedata/binarystaticinfo__/bin/wantedinfo.staticinfobody".into(),
        signature: None,
        changes: Vec::new(),
        fields: vec![Intent {
            entry: Some("target_01".into()),
            key: Some(key as u64),
            field: Some("_nonExistentField".into()),
            op: "set".into(),
            new: serde_json::json!(123),
            old: None,
            matches: None,
        }],
    };

    let res_err = check_patch_data(&invalid_patch, Some(&header), &body, true);
    assert_eq!(res_err.intents.len(), 1);
    assert!(matches!(res_err.intents[0], DryRunStatus::Failed(_)));
}

#[test]
fn package_mod_action_creates_distribution_archive() {
    let fx = fixture();
    let mut app = app(&fx);

    // 1. Create a staged JSON mod
    app.dispatch(Action::Mod(crate::ui::ModAction::CreateMod {
        name: "QuickLoot".into(),
        author: "Dev".into(),
        version: "1.0".into(),
        description: "Loot faster".into(),
    }));

    // 2. Trigger PackageModPrompt
    app.dispatch(Action::Mod(crate::ui::ModAction::PackageModPrompt("QuickLoot".into())));
    let draft = app.mods.package_mod.as_ref().expect("package draft");
    assert_eq!(draft.name, "QuickLoot");
    assert_eq!(draft.author, "Dev");
    assert_eq!(draft.version, "1.1"); // Auto-bumped

    // 3. Trigger PackageMod
    let meta = draft.clone();
    app.dispatch(Action::Mod(crate::ui::ModAction::PackageMod(meta)));
    assert!(app.mods.package_mod.is_none());

    // 4. Verify distribution archive in exports/
    let zip_path = app.paths.exports.join("QuickLoot-v1.1.zip");
    assert!(zip_path.exists());
    assert!(fs::metadata(&zip_path).unwrap().len() > 0);
}

#[test]
fn rebase_format2_offset_after_table_move() {
    use crate::patch::dry_run::{DryRunStatus, check_patch_data};
    use crate::patch::{Anchor, Change, Edit, FilePatch};

    // Body with entry "Potion" followed by payload: 0xaa, 0xbb at offset +4 instead of +0
    let key = 10u16;
    let name = "Potion";
    let payload = vec![0x00, 0x00, 0x00, 0x00, 0xaa, 0xbb]; // bytes moved 4 bytes ahead!

    let body = [
        &key.to_le_bytes()[..],
        &(name.len() as u32).to_le_bytes(),
        name.as_bytes(),
        &payload,
    ]
    .concat();

    let header = [
        &1u16.to_le_bytes()[..],
        &key.to_le_bytes(),
        &0u32.to_le_bytes(),
    ]
    .concat();

    // Patch assumed rel_offset was 0
    let patch = FilePatch {
        game_file: "test.staticinfobody".into(),
        signature: None,
        changes: vec![Change {
            anchor: Anchor::Entry {
                name: "Potion".into(),
                rel_offset: 0,
            },
            edit: Edit::Replace {
                original: Some(vec![0xaa, 0xbb]),
                patched: vec![0x11, 0x22],
            },
            label: "potion effect".into(),
        }],
        fields: Vec::new(),
    };

    let res = check_patch_data(&patch, Some(&header), &body, true);
    assert_eq!(res.changes.len(), 1);
    assert!(matches!(res.changes[0], DryRunStatus::Failed(_)));
    // Should detect rebase suggestion at +4
    let sug = res.rebases.get(&0).expect("rebase suggestion");
    assert_eq!(sug.entry_name, "Potion");
    assert_eq!(sug.old_offset, 0);
    assert_eq!(sug.new_offset, 4);

    // Apply rebase rewrite
    let original_json = r#"{
      "format": 2,
      "patches": [{
        "game_file": "test.staticinfobody",
        "changes": [{
          "entry": "Potion",
          "rel_offset": 0,
          "original": "aabb",
          "patched": "1122"
        }]
      }]
    }"#;

    let rebased_json = crate::patch::document::rebase_offset(original_json, 0, 0, sug.new_offset).unwrap();
    assert!(rebased_json.contains("\"rel_offset\": 4"));
}

#[test]
fn spawn_project_modal_action_creates_project_and_stages() {
    let fx = fixture();
    let mut app = app(&fx);
    let temp = tempfile::tempdir().unwrap();

    // 1. Trigger SpawnProjectPrompt
    app.dispatch(Action::Mod(crate::ui::ModAction::SpawnProjectPrompt));
    assert!(app.mods.spawn_project.is_some());

    // 2. Dispatch SpawnProject
    let draft = crate::ui::SpawnProjectDraft {
        name: "HudMinimap".into(),
        mod_type: crate::sdk::ProjectType::Asi,
        location: temp.path().display().to_string(),
        author: "DevHud".into(),
        version: "1.0".into(),
        description: "Custom HUD minimap".into(),
        stage_after: true,
    };
    app.dispatch(Action::Mod(crate::ui::ModAction::SpawnProject(draft)));
    assert!(app.mods.spawn_project.is_none());

    // 3. Verify project folder on disk
    let proj = temp.path().join("HudMinimap");
    assert!(proj.is_dir());
    assert!(proj.join("Cargo.toml").is_file());
    assert!(proj.join("src/lib.rs").is_file());
    assert!(proj.join("config.ini").is_file());
    assert!(proj.join("README.md").is_file());
    assert!(proj.join("DEVELOPMENT.md").is_file());

    // 4. Verify staged in Pailune
    assert!(app.paths.staging.join("HudMinimap").is_dir());
    assert_eq!(app.mods.selected.as_deref(), Some("HudMinimap"));
}
