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

/// Keeps only the newest `keep` events (run at start-up).
pub fn trim(conn: &Connection, keep: usize) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM events WHERE id <= (SELECT id FROM events ORDER BY id DESC LIMIT 1 OFFSET ?1)",
        [keep as i64],
    )?)
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

#[cfg(test)]
mod tests {
    use crate::logging::{Event, EventKind, Severity};

    #[test]
    fn trims_to_the_newest_events() {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::db::open(&dir.path().join("e.sqlite3")).unwrap();
        (0..10).for_each(|i| {
            super::insert(
                &conn,
                &Event::new(Severity::Info, EventKind::Job, None, format!("e{i}")),
            )
            .unwrap()
        });
        assert_eq!(super::trim(&conn, 3).unwrap(), 7);
        let left: Vec<_> = super::recent(&conn, 10, None)
            .unwrap()
            .into_iter()
            .map(|e| e.message)
            .collect();
        assert_eq!(left, ["e9", "e8", "e7"]);
        assert_eq!(super::trim(&conn, 3).unwrap(), 0);
    }
}
