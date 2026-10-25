//! The `events` table: a queryable journal for debugging.

use crate::error::Result;
use crate::logging::{Event, EventKind, Severity};
use rusqlite::{Connection, params};

/// Appends an event.
pub fn insert(conn: &Connection, event: &Event) -> Result<()> {
    conn.execute(
        "INSERT INTO events (ts, severity, kind, mod_name, message) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            event.ts,
            event.severity.as_str(),
            event.kind.as_str(),
            event.mod_name,
            event.message
        ],
    )?;
    Ok(())
}

/// Most recent events first, optionally only those concerning one mod.
pub fn recent(conn: &Connection, limit: usize, mod_name: Option<&str>) -> Result<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT ts, severity, kind, mod_name, message FROM events
         WHERE (?1 IS NULL OR mod_name = ?1) ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![mod_name, limit as i64], |row| {
        Ok(Event {
            ts: row.get(0)?,
            severity: Severity::parse(&row.get::<_, String>(1)?),
            kind: EventKind::parse(&row.get::<_, String>(2)?),
            mod_name: row.get(3)?,
            message: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
