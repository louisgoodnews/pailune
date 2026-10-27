//! SQLite persistence: Pailune's application memory.
//!
//! Every thread opens its own [`Connection`]; WAL mode and a busy timeout let
//! background jobs and the UI read and write concurrently.

pub mod collections;
pub mod events;
pub mod links;
pub mod mods;
pub mod profiles;
mod schema;

use crate::error::Result;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

/// Serialises connection setup inside this process: SQLite answers a
/// concurrent switch to WAL mode with `SQLITE_BUSY` without consulting the
/// busy handler, which happens when several jobs open a fresh database.
static SETUP: Mutex<()> = Mutex::new(());

/// Opens (and migrates) the database at `path`.
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(10))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let _guard = SETUP
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    conn.pragma_update(None, "journal_mode", "WAL")?;
    schema::migrate(&mut conn)?;
    Ok(conn)
}

/// Brings a (restored) database to the current schema.
pub fn schema_upgrade(conn: &mut Connection) -> Result<()> {
    schema::migrate(conn)
}

/// Reads a value from the key/value table.
pub fn get_value(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM kv WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}

/// Writes (or with `None`, deletes) a value in the key/value table.
pub fn set_value(conn: &Connection, key: &str, value: Option<&str>) -> Result<()> {
    match value {
        Some(value) => conn.execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?,
        None => conn.execute("DELETE FROM kv WHERE key = ?1", [key])?,
    };
    Ok(())
}

/// Current Unix time in seconds.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    /// Several threads opening a fresh database at once must all succeed.
    #[test]
    fn concurrent_first_open() {
        (0..20).for_each(|_| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("x.sqlite3");
            let handles: Vec<_> = (0..6)
                .map(|_| {
                    let path = path.clone();
                    std::thread::spawn(move || {
                        super::open(&path).map(drop).map_err(|e| e.to_string())
                    })
                })
                .collect();
            let errors: Vec<_> = handles
                .into_iter()
                .filter_map(|h| h.join().unwrap().err())
                .collect();
            assert!(errors.is_empty(), "{errors:?}");
        });
    }
}
