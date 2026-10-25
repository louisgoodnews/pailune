//! Backup and restore of Pylune's own state.

use super::{enabled_registry, fixture};
use crate::app_backup::{BackupKind, create, restore};
use crate::config::paths::DataPaths;
use crate::db;
use crate::mod_engine::install;
use crate::profiles::{Profile, ProfileSlot};
use std::fs;

#[test]
fn full_backups_restore_mods_profiles_and_settings() {
    let fx = fixture();
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    install::import(&fx.sources.join("Cool Mod"), &fx.staging).unwrap();
    let mut conn = db::open(&paths.database).unwrap();
    db::mods::update(&mut conn, |_| Ok(enabled_registry(&fx))).unwrap();
    let profile = Profile {
        name: "Main".into(),
        slots: vec![ProfileSlot {
            name: "Cool Mod".into(),
            enabled: true,
        }],
        launch_options: "-dx12".into(),
    };
    db::profiles::save(&mut conn, &profile).unwrap();
    fs::write(&paths.config_file, "[game]\napp_id = 3321460\n").unwrap();

    let full = create::create(&paths, &conn, BackupKind::Full, "manual", 100).unwrap();
    let light = create::create(&paths, &conn, BackupKind::Settings, create_label(), 200).unwrap();
    assert!(full.size > light.size, "full backups carry the mod files");
    assert_eq!((full.mods, full.kind), (1, BackupKind::Full));
    assert_eq!(create::list(&paths.backups).len(), 2);

    // Lose everything, then restore.
    db::mods::update(&mut conn, |_| Ok(Default::default())).unwrap();
    db::profiles::delete(&conn, "Main").unwrap();
    fs::remove_dir_all(fx.staging.join("Cool Mod")).unwrap();
    fs::write(&paths.config_file, "broken = [").unwrap();
    let report = restore::restore(&paths, &mut conn, &full.path, 300).unwrap();
    assert!(report.settings && report.mod_files);
    assert!(
        report.previous_staging.as_ref().is_some_and(|p| p.is_dir()),
        "old staging kept aside"
    );
    assert_eq!(db::mods::load(&conn).unwrap().mods[0].name, "Cool Mod");
    assert!(db::mods::load(&conn).unwrap().mods[0].enabled);
    assert_eq!(db::profiles::load(&conn).unwrap().profiles, vec![profile]);
    assert!(fx.staging.join("Cool Mod/CoolMod.asi").is_file());
    assert!(
        fs::read_to_string(&paths.config_file)
            .unwrap()
            .contains("3321460")
    );
    assert!(
        db::links::load(&conn).unwrap().links.is_empty(),
        "no stale deployment records"
    );

    // Automatic backups are pruned, manual ones are not.
    (0..4).for_each(|i| {
        drop(
            create::create(
                &paths,
                &conn,
                BackupKind::Settings,
                crate::app_backup::AUTO_LABEL,
                400 + i,
            )
            .unwrap(),
        )
    });
    assert_eq!(create::prune_auto(&paths.backups, 2).unwrap(), 2);
    assert_eq!(create::list(&paths.backups).len(), 4);
    assert!(!create::auto_due(&paths.backups, 403 + 3600));
    assert!(create::auto_due(&paths.backups, 403 + 25 * 3600));
    fs::write(paths.backups.join("junk.zip"), "not a zip").unwrap();
    assert_eq!(
        create::list(&paths.backups).len(),
        4,
        "foreign files are ignored"
    );
}

fn create_label() -> &'static str {
    "settings only"
}
