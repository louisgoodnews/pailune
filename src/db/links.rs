//! The deployment manifest (`links`, `created_dirs`).

use crate::error::Result;
use crate::symlink::{FileStamp, LinkRecord, Manifest};
use rusqlite::{Connection, params};
use std::path::PathBuf;

/// Loads the manifest of currently deployed links.
pub fn load(conn: &Connection) -> Result<Manifest> {
    let links = conn
        .prepare("SELECT target, source, backup, stamp_size, stamp_mtime FROM links ORDER BY seq")?
        .query_map([], |row| {
            let size = row.get::<_, Option<i64>>(3)?;
            let mtime = row.get::<_, Option<i64>>(4)?;
            Ok(LinkRecord {
                target: PathBuf::from(row.get::<_, String>(0)?),
                source: PathBuf::from(row.get::<_, String>(1)?),
                backup: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
                stamp: size.zip(mtime).map(|(size, modified_ns)| FileStamp {
                    size: size as u64,
                    modified_ns,
                }),
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

/// Re-stamps the hard links of `source` after Pylune itself rewrote it: a
/// hard link shares the new contents, so its old stamp would make the next
/// undeploy treat the file as the user's own. Copies keep their stamp.
pub fn restamp_shared(conn: &Connection, source: &std::path::Path) -> Result<()> {
    use crate::symlink::journal::Journal;
    let fresh = crate::symlink::stamp(source);
    load(conn)?
        .links
        .iter()
        .filter(|r| r.stamp.is_some() && r.source == source)
        .filter(|r| fresh.is_some() && crate::symlink::stamp(&r.target) == fresh)
        .try_for_each(|r| {
            let stamp = fresh.expect("checked above");
            DbJournal { conn }.link_stamped(&r.target, &stamp)
        })
}

/// Replaces the stored manifest (tests; jobs use [`DbJournal`]).
#[cfg(test)]
pub fn save(conn: &mut Connection, manifest: &Manifest) -> Result<()> {
    let lossy = |p: &PathBuf| p.to_string_lossy().into_owned();
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM links", [])?;
    tx.execute("DELETE FROM created_dirs", [])?;
    manifest.links.iter().try_for_each(|r| {
        tx.execute(
            "INSERT INTO links (target, source, backup, stamp_size, stamp_mtime)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                lossy(&r.target),
                lossy(&r.source),
                r.backup.as_ref().map(lossy),
                r.stamp.map(|s| s.size as i64),
                r.stamp.map(|s| s.modified_ns)
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

/// Persists manifest changes to the database as they happen.
pub struct DbJournal<'a> {
    pub conn: &'a Connection,
}

impl crate::symlink::journal::Journal for DbJournal<'_> {
    fn link_planned(&mut self, record: &LinkRecord) -> Result<()> {
        let lossy = |p: &PathBuf| p.to_string_lossy().into_owned();
        self.conn.execute(
            "INSERT INTO links (target, source, backup) VALUES (?1, ?2, ?3)
             ON CONFLICT(target) DO UPDATE SET source = excluded.source, backup = excluded.backup,
             stamp_size = NULL, stamp_mtime = NULL",
            params![
                lossy(&record.target),
                lossy(&record.source),
                record.backup.as_ref().map(lossy)
            ],
        )?;
        Ok(())
    }

    fn link_stamped(&mut self, target: &std::path::Path, stamp: &FileStamp) -> Result<()> {
        self.conn.execute(
            "UPDATE links SET stamp_size = ?2, stamp_mtime = ?3 WHERE target = ?1",
            params![
                target.to_string_lossy(),
                stamp.size as i64,
                stamp.modified_ns
            ],
        )?;
        Ok(())
    }

    fn link_reverted(&mut self, target: &std::path::Path) -> Result<()> {
        self.conn.execute(
            "DELETE FROM links WHERE target = ?1",
            [target.to_string_lossy()],
        )?;
        Ok(())
    }

    fn dir_planned(&mut self, dir: &std::path::Path) -> Result<()> {
        self.conn.execute(
            "INSERT INTO created_dirs (path) VALUES (?1)",
            [dir.to_string_lossy()],
        )?;
        Ok(())
    }

    fn dirs_cleared(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM created_dirs", [])?;
        Ok(())
    }
}
