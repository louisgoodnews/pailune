//! The deployment manifest (`links`, `created_dirs`).

use crate::error::Result;
use crate::symlink::{LinkRecord, Manifest};
use rusqlite::{Connection, params};
use std::path::PathBuf;

/// Loads the manifest of currently deployed links.
pub fn load(conn: &Connection) -> Result<Manifest> {
    let links = conn
        .prepare("SELECT target, source, backup FROM links ORDER BY seq")?
        .query_map([], |row| {
            Ok(LinkRecord {
                target: PathBuf::from(row.get::<_, String>(0)?),
                source: PathBuf::from(row.get::<_, String>(1)?),
                backup: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let created_dirs = conn
        .prepare("SELECT path FROM created_dirs ORDER BY seq")?
        .query_map([], |row| row.get::<_, String>(0).map(PathBuf::from))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Manifest {
        links,
        created_dirs,
    })
}

/// Replaces the stored manifest.
pub fn save(conn: &mut Connection, manifest: &Manifest) -> Result<()> {
    let lossy = |p: &PathBuf| p.to_string_lossy().into_owned();
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM links", [])?;
    tx.execute("DELETE FROM created_dirs", [])?;
    manifest.links.iter().try_for_each(|r| {
        tx.execute(
            "INSERT INTO links (target, source, backup) VALUES (?1, ?2, ?3)",
            params![
                lossy(&r.target),
                lossy(&r.source),
                r.backup.as_ref().map(lossy)
            ],
        )
        .map(|_| ())
    })?;
    manifest.created_dirs.iter().try_for_each(|dir| {
        tx.execute("INSERT INTO created_dirs (path) VALUES (?1)", [lossy(dir)])
            .map(|_| ())
    })?;
    tx.commit()?;
    Ok(())
}
