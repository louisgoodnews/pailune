//! Profiles and their slots.

use crate::error::Result;
use crate::profiles::{Profile, ProfileBook, ProfileSlot};
use rusqlite::{Connection, params};

const ACTIVE_KEY: &str = "active_profile";

/// Loads every profile (sorted by name) and the active one.
pub fn load(conn: &Connection) -> Result<ProfileBook> {
    let heads: Vec<(String, String)> = conn
        .prepare("SELECT name, launch_options FROM profiles ORDER BY name COLLATE NOCASE")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut slots = conn.prepare(
        "SELECT mod_name, enabled FROM profile_slots WHERE profile = ?1 ORDER BY position",
    )?;
    let profiles = heads
        .into_iter()
        .map(|(name, launch_options)| {
            let slots = slots
                .query_map([&name], |row| {
                    Ok(ProfileSlot {
                        name: row.get(0)?,
                        enabled: row.get(1)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            Ok(Profile {
                name,
                slots,
                launch_options,
            })
        })
        .collect::<Result<_>>()?;
    Ok(ProfileBook {
        profiles,
        active: super::get_value(conn, ACTIVE_KEY)?,
    })
}

/// Inserts or replaces a profile.
pub fn save(conn: &mut Connection, profile: &Profile) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO profiles (name, launch_options) VALUES (?1, ?2)
         ON CONFLICT(name) DO UPDATE SET launch_options = excluded.launch_options",
        params![profile.name, profile.launch_options],
    )?;
    tx.execute(
        "DELETE FROM profile_slots WHERE profile = ?1",
        [&profile.name],
    )?;
    profile.slots.iter().enumerate().try_for_each(|(position, slot)| {
        tx.execute(
            "INSERT INTO profile_slots (profile, position, mod_name, enabled) VALUES (?1, ?2, ?3, ?4)",
            params![profile.name, position as i64, slot.name, slot.enabled],
        )
        .map(|_| ())
    })?;
    tx.commit()?;
    Ok(())
}

/// Deletes a profile (its slots cascade).
pub fn delete(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("DELETE FROM profiles WHERE name = ?1", [name])?;
    if super::get_value(conn, ACTIVE_KEY)?.as_deref() == Some(name) {
        super::set_value(conn, ACTIVE_KEY, None)?;
    }
    Ok(())
}

/// Marks a profile as active.
pub fn set_active(conn: &Connection, name: Option<&str>) -> Result<()> {
    super::set_value(conn, ACTIVE_KEY, name)
}
