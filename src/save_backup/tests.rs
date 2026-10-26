use super::*;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn backs_up_restores_and_prunes() {
    let root = tempfile::tempdir().unwrap();
    let saves = save_dir(Some(&root.path().join("pfx"))).unwrap();
    let dest = root.path().join("save-backups");
    assert!(
        create(&saves, &dest, "manual", 1).is_err(),
        "nothing to back up"
    );
    write(&saves.join("42/slot1/save.save"), "first");
    write(&saves.join("42/slot1/lobby.save"), "lobby");
    let backup = create(&saves, &dest, "manual", 10).unwrap();
    assert_eq!((backup.files, backup.label.as_str()), (2, "manual"));
    write(&saves.join("42/slot1/save.save"), "later");
    write(&saves.join("42/slot2/save.save"), "new slot");
    let safety = restore(&backup.path, &saves, &dest, 20).unwrap().unwrap();
    assert_eq!(
        fs::read_to_string(saves.join("42/slot1/save.save")).unwrap(),
        "first"
    );
    assert!(
        !saves.join("42/slot2").exists(),
        "the folder is replaced, not merged"
    );
    assert_eq!(safety.files, 3);
    let parent = saves.parent().unwrap();
    let leftovers = fs::read_dir(parent).unwrap().count();
    assert_eq!(leftovers, 1, "no work folders left");
    (0..4).for_each(|i| drop(create(&saves, &dest, AUTO_LABEL, 100 + i).unwrap()));
    assert_eq!(prune_auto(&dest, 2).unwrap(), 2);
    let labels: Vec<_> = list(&dest).into_iter().map(|b| b.label).collect();
    assert_eq!(labels, [AUTO_LABEL, AUTO_LABEL, "before restore", "manual"]);
    assert!(restore(&root.path().join("nope.zip"), &saves, &dest, 1).is_err());
}

#[test]
fn interrupted_restores_are_recovered_and_empty_folders_do_not_block() {
    let root = tempfile::tempdir().unwrap();
    let saves = save_dir(Some(&root.path().join("pfx"))).unwrap();
    let dest = root.path().join("backups");
    write(&saves.join("42/slot1/save.save"), "mine");
    let backup = create(&saves, &dest, "manual", 1).unwrap();
    // A crash between the two renames left the saves set aside.
    let parent = saves.parent().unwrap();
    fs::rename(&saves, parent.join(".pylune-old-saves-7")).unwrap();
    fs::create_dir_all(parent.join(".pylune-restore-7")).unwrap();
    recover(&saves).unwrap();
    assert_eq!(
        fs::read_to_string(saves.join("42/slot1/save.save")).unwrap(),
        "mine"
    );
    assert_eq!(
        fs::read_dir(parent).unwrap().count(),
        1,
        "work folder removed"
    );
    // Only empty folders: restoring still works (nothing to back up first).
    fs::remove_dir_all(&saves).unwrap();
    fs::create_dir_all(saves.join("42")).unwrap();
    assert!(restore(&backup.path, &saves, &dest, 2).unwrap().is_none());
    assert!(saves.join("42/slot1/save.save").is_file());
}
