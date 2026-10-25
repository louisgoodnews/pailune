//! Installed Nexus collections and their members.

use crate::error::Result;
use crate::nexus::collection::{Collection, Member};
use rusqlite::{Connection, params};

/// Loads every installed collection with its members.
pub fn load(conn: &Connection) -> Result<Vec<Collection>> {
    let heads: Vec<(String, String, u32)> = conn
        .prepare("SELECT slug, name, revision FROM collections ORDER BY name COLLATE NOCASE")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt = conn.prepare(
        "SELECT position, mod_id, file_id, optional, selected, title, size, mod_name
         FROM collection_members WHERE slug = ?1 ORDER BY position",
    )?;
    heads
        .into_iter()
        .map(|(slug, name, revision)| {
            let members = stmt
                .query_map([&slug], |r| {
                    Ok(Member {
                        position: r.get::<_, i64>(0)? as usize,
                        mod_id: r.get::<_, i64>(1)? as u64,
                        file_id: r.get::<_, i64>(2)? as u64,
                        optional: r.get(3)?,
                        selected: r.get(4)?,
                        title: r.get(5)?,
                        size: r.get::<_, i64>(6)? as u64,
                        mod_name: r.get(7)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            Ok(Collection {
                slug,
                name,
                revision,
                members,
            })
        })
        .collect()
}

/// Stores (or replaces) a collection; installed member names are kept by
/// mod id so a revision update does not forget what is staged.
pub fn save(conn: &mut Connection, collection: &Collection) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO collections (slug, name, revision, installed_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(slug) DO UPDATE SET name = excluded.name, revision = excluded.revision",
        params![
            collection.slug,
            collection.name,
            collection.revision,
            crate::db::now()
        ],
    )?;
    tx.execute(
        "DELETE FROM collection_members WHERE slug = ?1",
        [&collection.slug],
    )?;
    collection.members.iter().try_for_each(|m| {
        tx.execute(
            "INSERT INTO collection_members (slug, position, mod_id, file_id, optional, selected, title, size, mod_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![collection.slug, m.position as i64, m.mod_id as i64, m.file_id as i64, m.optional, m.selected, m.title, m.size as i64, m.mod_name],
        )
        .map(|_| ())
    })?;
    tx.commit()?;
    Ok(())
}

/// Records that a member's file was installed as staged mod `mod_name`.
pub fn link_member(
    conn: &Connection,
    slug: &str,
    mod_id: u64,
    file_id: u64,
    mod_name: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE collection_members SET mod_name = ?4 WHERE slug = ?1 AND mod_id = ?2 AND file_id = ?3",
        params![slug, mod_id as i64, file_id as i64, mod_name],
    )?;
    Ok(())
}

/// Forgets a collection (members cascade).
pub fn delete(conn: &Connection, slug: &str) -> Result<()> {
    conn.execute("DELETE FROM collections WHERE slug = ?1", [slug])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_collections_and_links_members() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = crate::db::open(&dir.path().join("c.sqlite3")).unwrap();
        let member = |position, mod_id, file_id| Member {
            position,
            mod_id,
            file_id,
            optional: false,
            selected: true,
            title: "t".into(),
            size: 1,
            mod_name: None,
        };
        let c = Collection {
            slug: "abc".into(),
            name: "QOL".into(),
            revision: 3,
            members: vec![member(0, 1, 10), member(1, 2, 20)],
        };
        save(&mut conn, &c).unwrap();
        link_member(&conn, "abc", 2, 20, "Two").unwrap();
        let loaded = load(&conn).unwrap();
        assert_eq!(loaded[0].members[1].mod_name.as_deref(), Some("Two"));
        assert_eq!((loaded[0].revision, loaded[0].members.len()), (3, 2));
        delete(&conn, "abc").unwrap();
        assert!(load(&conn).unwrap().is_empty());
    }
}
