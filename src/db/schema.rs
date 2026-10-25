//! Versioned schema migrations, tracked with `PRAGMA user_version`.

use crate::error::Result;
use rusqlite::{Connection, TransactionBehavior};

/// Migration `n` upgrades the schema from version `n` to `n + 1`.
const MIGRATIONS: [&str; 1] = [r#"
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
"#];

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
