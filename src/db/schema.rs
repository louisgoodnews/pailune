//! Versioned schema migrations, tracked with `PRAGMA user_version`.

use crate::error::Result;
use rusqlite::{Connection, TransactionBehavior};

/// Migration `n` upgrades the schema from version `n` to `n + 1`.
const MIGRATIONS: [&str; 5] = [
    r#"
    CREATE TABLE mods (
        name          TEXT PRIMARY KEY,
        position      INTEGER NOT NULL,
        enabled       INTEGER NOT NULL DEFAULT 0,
        kind          TEXT NOT NULL,
        file_count    INTEGER NOT NULL DEFAULT 0,
        installed_at  INTEGER NOT NULL,
        source        TEXT NOT NULL DEFAULT '',
        nexus_mod_id  INTEGER,
        nexus_file_id INTEGER,
        nexus_version TEXT
    );
    CREATE TABLE links (
        seq     INTEGER PRIMARY KEY,
        target  TEXT NOT NULL UNIQUE,
        source  TEXT NOT NULL,
        backup  TEXT
    );
    CREATE TABLE created_dirs (
        seq  INTEGER PRIMARY KEY,
        path TEXT NOT NULL
    );
    CREATE TABLE profiles (
        name           TEXT PRIMARY KEY,
        launch_options TEXT NOT NULL DEFAULT ''
    );
    CREATE TABLE profile_slots (
        profile  TEXT NOT NULL REFERENCES profiles(name) ON DELETE CASCADE,
        position INTEGER NOT NULL,
        mod_name TEXT NOT NULL,
        enabled  INTEGER NOT NULL,
        PRIMARY KEY (profile, position)
    );
    CREATE TABLE events (
        id       INTEGER PRIMARY KEY,
        ts       INTEGER NOT NULL,
        severity TEXT NOT NULL,
        kind     TEXT NOT NULL,
        mod_name TEXT,
        message  TEXT NOT NULL
    );
    CREATE INDEX events_by_mod ON events(mod_name, id);
    CREATE TABLE kv (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
"#,
    r#"
    ALTER TABLE mods ADD COLUMN paz_files INTEGER NOT NULL DEFAULT 0;
"#,
    r#"
    CREATE TABLE collections (
        slug         TEXT PRIMARY KEY,
        name         TEXT NOT NULL,
        revision     INTEGER NOT NULL,
        installed_at INTEGER NOT NULL
    );
    CREATE TABLE collection_members (
        slug     TEXT NOT NULL REFERENCES collections(slug) ON DELETE CASCADE,
        position INTEGER NOT NULL,
        mod_id   INTEGER NOT NULL,
        file_id  INTEGER NOT NULL,
        optional INTEGER NOT NULL,
        selected INTEGER NOT NULL,
        title    TEXT NOT NULL,
        size     INTEGER NOT NULL DEFAULT 0,
        mod_name TEXT,
        PRIMARY KEY (slug, position)
    );
"#,
    r#"
    ALTER TABLE mods ADD COLUMN variant TEXT;
"#,
    r#"
    ALTER TABLE links ADD COLUMN stamp_size INTEGER;
    ALTER TABLE links ADD COLUMN stamp_mtime INTEGER;
"#,
];

/// Applies every migration newer than the database's `user_version`.
///
/// Runs inside one `BEGIN IMMEDIATE` transaction and reads the version only
/// after taking the write lock, so connections opened concurrently (several
/// jobs at first start) never apply a migration twice.
pub fn migrate(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let pending = MIGRATIONS
        .iter()
        .skip(usize::try_from(version).unwrap_or(0));
    pending.clone().try_for_each(|sql| tx.execute_batch(sql))?;
    if pending.count() > 0 {
        tx.pragma_update(None, "user_version", MIGRATIONS.len() as i64)?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::MIGRATIONS;

    /// A database created by an older Pailune keeps its rows when upgraded.
    #[test]
    fn upgrades_a_version_1_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.sqlite3");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(MIGRATIONS[0]).unwrap();
            conn.execute_batch(
                "PRAGMA user_version = 1;
                 INSERT INTO mods (name, position, enabled, kind, file_count, installed_at)
                 VALUES ('Old', 0, 1, 'asi', 2, 7);",
            )
            .unwrap();
        }
        let conn = crate::db::open(&path).unwrap();
        let registry = crate::db::mods::load(&conn).unwrap();
        assert_eq!(registry.mods.len(), 1);
        assert_eq!(
            (registry.mods[0].file_count, registry.mods[0].paz_files),
            (2, 0)
        );
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
    }
}
