//! The `mods` table: load order, enabled flags and install metadata.

use crate::error::Result;
use crate::mod_engine::model::{ModEntry, ModKind, NexusRef, Registry};
use rusqlite::{Connection, Row, TransactionBehavior, params};

/// Name of a mod kind in the database.
fn kind_to_str(kind: ModKind) -> &'static str {
    match kind {
        ModKind::Json => "json",
        ModKind::Raw => "raw",
        ModKind::Asi => "asi",
        ModKind::Overlay => "overlay",
        ModKind::Mixed => "mixed",
        ModKind::Empty => "empty",
    }
}

/// Mod kind from its database name (unknown names read as empty).
fn kind_from_str(text: &str) -> ModKind {
    match text {
        "json" => ModKind::Json,
        "raw" => ModKind::Raw,
        "asi" => ModKind::Asi,
        "overlay" => ModKind::Overlay,
        "mixed" => ModKind::Mixed,
        _ => ModKind::Empty,
    }
}

/// Builds a mod entry from a `mods` row.
fn entry_from_row(row: &Row) -> rusqlite::Result<ModEntry> {
    let nexus_mod: Option<i64> = row.get("nexus_mod_id")?;
    let nexus_file: Option<i64> = row.get("nexus_file_id")?;
    let nexus_version: Option<String> = row.get("nexus_version")?;
    Ok(ModEntry {
        name: row.get("name")?,
        kind: kind_from_str(&row.get::<_, String>("kind")?),
        enabled: row.get("enabled")?,
        file_count: row.get::<_, i64>("file_count")? as usize,
        paz_files: row.get::<_, i64>("paz_files")? as usize,
        variant: row.get("variant")?,
        installed_at: row.get("installed_at")?,
        source: row.get("source")?,
        nexus: nexus_mod.zip(nexus_file).map(|(m, f)| NexusRef {
            mod_id: m as u64,
            file_id: f as u64,
            version: nexus_version.unwrap_or_default(),
        }),
    })
}

/// Loads all mods in load order.
pub fn load(conn: &Connection) -> Result<Registry> {
    let mut stmt = conn.prepare("SELECT * FROM mods ORDER BY position")?;
    let mods = stmt
        .query_map([], entry_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Registry { mods })
}

/// Writes `registry` over the whole table (order = position).
fn write(conn: &Connection, registry: &Registry) -> Result<()> {
    conn.execute("DELETE FROM mods", [])?;
    registry
        .mods
        .iter()
        .enumerate()
        .try_for_each(|(position, m)| {
            conn.execute(
                "INSERT INTO mods (name, position, enabled, kind, file_count, installed_at, source,
                               nexus_mod_id, nexus_file_id, nexus_version, paz_files, variant)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    m.name,
                    position as i64,
                    m.enabled,
                    kind_to_str(m.kind),
                    m.file_count as i64,
                    m.installed_at,
                    m.source,
                    m.nexus.as_ref().map(|n| n.mod_id as i64),
                    m.nexus.as_ref().map(|n| n.file_id as i64),
                    m.nexus.as_ref().map(|n| n.version.clone()),
                    m.paz_files as i64,
                    m.variant,
                ],
            )
            .map(|_| ())
        })?;
    Ok(())
}

/// Atomically transforms the registry: read, apply `f`, write back.
///
/// `BEGIN IMMEDIATE` serialises concurrent writers (UI and jobs), so no
/// update is lost when several of them change the mod list at once.
pub fn update(
    conn: &mut Connection,
    f: impl FnOnce(&Registry) -> Result<Registry>,
) -> Result<Registry> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = load(&tx)?;
    let next = f(&current)?;
    if next != current {
        write(&tx, &next)?;
        bump_revision(&tx)?;
    }
    tx.commit()?;
    Ok(next)
}

/// Key/value entry counting changes to the mod list.
const REVISION_KEY: &str = "registry_revision";
/// Key/value entry holding the revision last deployed.
const DEPLOYED_KEY: &str = "deployed_revision";

/// Reads a revision counter from the key/value table.
fn revision(conn: &Connection, key: &str) -> Result<Option<i64>> {
    Ok(super::get_value(conn, key)?.and_then(|v| v.parse().ok()))
}

/// Marks the registry (or a staged mod's files) as changed.
fn bump_revision(conn: &Connection) -> Result<()> {
    let next = revision(conn, REVISION_KEY)?.unwrap_or(0) + 1;
    super::set_value(conn, REVISION_KEY, Some(&next.to_string()))
}

/// Loads the registry together with its revision, consistently.
pub fn load_with_revision(conn: &Connection) -> Result<(Registry, i64)> {
    let tx = conn.unchecked_transaction()?;
    let loaded = (load(&tx)?, revision(&tx, REVISION_KEY)?.unwrap_or(0));
    tx.commit()?;
    Ok(loaded)
}

/// Records which revision is deployed; `None` after unlinking everything.
pub fn set_deployed_revision(conn: &Connection, deployed: Option<i64>) -> Result<()> {
    super::set_value(
        conn,
        DEPLOYED_KEY,
        deployed.map(|r| r.to_string()).as_deref(),
    )
}

/// Whether the game directory lags behind the registry.
///
/// Nothing is pending when no mod is enabled and nothing is linked.
pub fn pending(conn: &Connection, registry: &Registry, links: usize) -> Result<bool> {
    let current = revision(conn, REVISION_KEY)?.unwrap_or(0);
    let deployed = revision(conn, DEPLOYED_KEY)?;
    let idle = links == 0 && registry.mods.iter().all(|m| !m.enabled);
    Ok(!idle && deployed != Some(current))
}

/// Records where a (re)installed mod came from.
pub fn record_install(
    conn: &Connection,
    name: &str,
    source: &str,
    nexus: Option<&NexusRef>,
    at: i64,
) -> Result<()> {
    conn.execute(
        "UPDATE mods SET source = ?2, installed_at = ?3, nexus_mod_id = ?4,
                         nexus_file_id = ?5, nexus_version = ?6 WHERE name = ?1",
        params![
            name,
            source,
            at,
            nexus.map(|n| n.mod_id as i64),
            nexus.map(|n| n.file_id as i64),
            nexus.map(|n| n.version.clone()),
        ],
    )?;
    // New files of an installed mod need a redeploy even if the list is unchanged.
    bump_revision(conn)
}
