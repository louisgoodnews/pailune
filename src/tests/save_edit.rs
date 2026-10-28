//! Editing a save through the UI: typed value, confirmation, exclusive job
//! with a backup first, and the viewer showing the new value.

use super::fixture;
use super::ui::{app, settle};
use crate::savefile::edit::tests::pouch_save;
use crate::ui::{Action, BackupAction, Confirm, Level};

#[test]
fn save_values_are_edited_after_a_backup() {
    let fx = fixture();
    let mut app = app(&fx);
    settle(&mut app);
    let saves = crate::save_backup::save_dir(app.info.prefix_dir.as_deref()).unwrap();
    let slot = saves.join("7/slot1");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(slot.join("save.save"), pouch_save()).unwrap();
    app.dispatch(Action::Backup(BackupAction::InspectSave(slot.clone())));
    let money = |app: &crate::ui::PailuneApp| {
        let view = app.save_view.as_ref().unwrap();
        view.tree[0].children[0].edit.clone().unwrap()
    };
    let at = money(&app).at;
    // An invalid value is refused before anything is asked.
    app.save_view
        .as_mut()
        .unwrap()
        .edits
        .insert(at, "-5".into());
    app.dispatch(Action::Backup(BackupAction::SaveEdits));
    assert_eq!(app.status.level, Level::Error, "{:?}", app.status);
    assert!(app.confirm.is_none());

    app.save_view
        .as_mut()
        .unwrap()
        .edits
        .insert(at, "350".into());
    app.dispatch(Action::Backup(BackupAction::SaveEdits));
    assert_eq!(app.confirm, Some(Confirm::EditSave(1)));
    app.dispatch(Action::Backup(BackupAction::WriteSaveEdits));
    settle(&mut app);
    assert_eq!(app.status.level, Level::Success, "{:?}", app.status);
    assert_eq!(
        crate::savefile::view::scalar("uint64", &money(&app).bytes),
        "350",
        "the viewer shows the saved value"
    );
    assert!(app.save_view.as_ref().unwrap().edits.is_empty());
    assert_eq!(app.save_backups.len(), 1, "all saves backed up first");
    assert_eq!(app.save_backups[0].label, "before-edit");
}

#[test]
fn list_items_can_be_duplicated_and_deleted_through_ui() {
    let fx = fixture();
    let mut app = app(&fx);
    settle(&mut app);
    let saves = crate::save_backup::save_dir(app.info.prefix_dir.as_deref()).unwrap();
    let slot = saves.join("7/slot2");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(
        slot.join("save.save"),
        crate::savefile::edit::tests::inventory_save(),
    )
    .unwrap();
    app.dispatch(Action::Backup(BackupAction::InspectSave(slot.clone())));

    let view = app.save_view.as_ref().unwrap();
    let list_node = &view.tree[0].children[1];
    assert_eq!(list_node.children.len(), 2);
    let item0 = list_node.children[0].list_item.as_ref().unwrap();

    // Duplicate item 0
    app.dispatch(Action::Backup(BackupAction::DuplicateListItem {
        list_at: item0.list_at,
        index: item0.index,
        label: "duplicate item #0".into(),
    }));
    assert_eq!(app.save_view.as_ref().unwrap().list_ops.len(), 1);

    // Save changes
    app.dispatch(Action::Backup(BackupAction::SaveEdits));
    assert_eq!(app.confirm, Some(Confirm::EditSave(2)));
    app.dispatch(Action::Backup(BackupAction::WriteSaveEdits));
    settle(&mut app);

    assert_eq!(app.status.level, Level::Success, "{:?}", app.status);
    let updated_view = app.save_view.as_ref().unwrap();
    let updated_list = &updated_view.tree[0].children[1];
    assert_eq!(updated_list.children.len(), 3, "item was duplicated");

    // Delete item 1
    let item1 = updated_list.children[1].list_item.as_ref().unwrap();
    app.dispatch(Action::Backup(BackupAction::DeleteListItem {
        list_at: item1.list_at,
        index: item1.index,
        label: "delete item #1".into(),
    }));
    app.dispatch(Action::Backup(BackupAction::SaveEdits));
    assert_eq!(app.confirm, Some(Confirm::EditSave(2)));
    app.dispatch(Action::Backup(BackupAction::WriteSaveEdits));
    settle(&mut app);

    assert_eq!(app.status.level, Level::Success, "{:?}", app.status);
    let final_view = app.save_view.as_ref().unwrap();
    let final_list = &final_view.tree[0].children[1];
    assert_eq!(final_list.children.len(), 2, "item was deleted");
}
