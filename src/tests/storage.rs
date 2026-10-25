//! SQLite persistence.

use super::fixture;
use crate::mod_engine::model::ModKind;
use crate::mod_engine::{install, registry};
use crate::symlink::Manifest;

#[test]
fn database_round_trips_state() {
    use crate::db;
    use crate::logging::{Event, EventKind, Severity};
    use crate::mod_engine::model::NexusRef;
    use crate::profiles::{Profile, ProfileSlot};
    use crate::symlink::LinkRecord;
    let fx = fixture();
    install::import(&fx.sources.join("Overlay"), &fx.staging).unwrap();
    let mut conn = db::open(&fx.staging.with_file_name("test.sqlite3")).unwrap();
    let staging = fx.staging.clone();
    let reg = db::mods::update(&mut conn, |r| registry::sync(r, &staging, 5)).unwrap();
    assert_eq!(reg.mods[0].kind, ModKind::Asi);
    let origin = NexusRef {
        mod_id: 1,
        file_id: 2,
        version: "1.0".into(),
    };
    db::mods::record_install(&conn, "Overlay", "nexus:x", Some(&origin), 9).unwrap();
    let loaded = db::mods::load(&conn).unwrap();
    assert_eq!(loaded.mods[0].nexus.as_ref(), Some(&origin));
    assert_eq!(
        (loaded.mods[0].installed_at, loaded.mods[0].source.as_str()),
        (9, "nexus:x")
    );

    let manifest = Manifest {
        links: vec![LinkRecord {
            target: "/g/a".into(),
            source: "/s/a".into(),
            backup: Some("/g/a.bak".into()),
        }],
        created_dirs: vec!["/g/d".into()],
    };
    db::links::save(&mut conn, &manifest).unwrap();
    assert_eq!(db::links::load(&conn).unwrap(), manifest);

    let profile = Profile {
        name: "Main".into(),
        slots: vec![ProfileSlot {
            name: "Overlay".into(),
            enabled: true,
        }],
        launch_options: "-dx12".into(),
    };
    db::profiles::save(&mut conn, &profile).unwrap();
    db::profiles::set_active(&conn, Some("Main")).unwrap();
    let book = db::profiles::load(&conn).unwrap();
    assert_eq!(
        (book.profiles, book.active.as_deref()),
        (vec![profile], Some("Main"))
    );
    db::profiles::delete(&conn, "Main").unwrap();
    assert_eq!(db::profiles::load(&conn).unwrap().active, None);

    let event = Event::new(Severity::Warn, EventKind::Install, Some("Overlay"), "hello");
    db::events::insert(&conn, &event).unwrap();
    assert_eq!(
        db::events::recent(&conn, 10, Some("Overlay")).unwrap(),
        vec![event]
    );
    assert!(
        db::events::recent(&conn, 10, Some("Other"))
            .unwrap()
            .is_empty()
    );
}
