//! Recoverable deletes: removed mods move to a trash folder with their
//! metadata and can be restored until the trash is pruned.

use super::model::{ModEntry, NexusRef};
use crate::error::{IoContext, PailuneError, Result};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

/// A deleted mod waiting in the trash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashItem {
    /// Folder name inside the trash (`<unix time>-<mod name>`).
    pub id: String,
    pub name: String,
    pub deleted_at: i64,
    pub source: String,
    pub nexus: Option<NexusRef>,
    pub variant: Option<String>,
}

/// Metadata stored with a deleted mod (`entry.json`): name, time, origin, variant.
fn metadata(entry: &ModEntry, deleted_at: i64) -> Value {
    json!({
        "name": entry.name,
        "deleted_at": deleted_at,
        "source": entry.source,
        "variant": entry.variant,
        "nexus": entry.nexus.as_ref().map(|n| json!({"mod_id": n.mod_id, "file_id": n.file_id, "version": n.version})),
    })
}

/// Reads one trash entry, if it is complete.
fn read_item(dir: &Path) -> Option<TrashItem> {
    let meta: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("entry.json")).ok()?).ok()?;
    let text = |k: &str| meta.get(k).and_then(Value::as_str).map(str::to_string);
    let nexus = meta.get("nexus").filter(|n| !n.is_null()).and_then(|n| {
        Some(NexusRef {
            mod_id: n.get("mod_id")?.as_u64()?,
            file_id: n.get("file_id")?.as_u64()?,
            version: n
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
    });
    Some(TrashItem {
        id: dir.file_name()?.to_string_lossy().into_owned(),
        name: text("name")?,
        deleted_at: meta.get("deleted_at").and_then(Value::as_i64).unwrap_or(0),
        source: text("source").unwrap_or_default(),
        nexus,
        variant: text("variant"),
    })
}

/// Moves a staged mod into the trash (a rename: fast and atomic).
pub fn move_to_trash(
    staging: &Path,
    trash: &Path,
    entry: &ModEntry,
    now: i64,
) -> Result<TrashItem> {
    let id = format!("{now}-{}", entry.name);
    let dir = trash.join(&id);
    fs::create_dir_all(&dir).at(&dir)?;
    let from = staging.join(&entry.name);
    fs::rename(&from, dir.join("mod")).at(&from)?;
    fs::write(dir.join("entry.json"), metadata(entry, now).to_string()).at(&dir)?;
    read_item(&dir).ok_or_else(|| PailuneError::Invalid("trash entry unreadable".into()))
}

/// Trash entries, newest first.
pub fn list(trash: &Path) -> Vec<TrashItem> {
    let mut items: Vec<TrashItem> = fs::read_dir(trash)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| read_item(&e.path()))
                .collect()
        })
        .unwrap_or_default();
    items.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at).then(b.id.cmp(&a.id)));
    items
}

/// Moves a trashed mod back into staging, under a free name. Returns the
/// staged name and the item's metadata.
pub fn restore(trash: &Path, staging: &Path, id: &str) -> Result<(String, TrashItem)> {
    let dir = trash.join(id);
    let item = read_item(&dir)
        .ok_or_else(|| PailuneError::Invalid(format!("nothing to restore for {id}")))?;
    let name = (1..)
        .map(|i| match i {
            1 => item.name.clone(),
            i => format!("{} ({i})", item.name),
        })
        .find(|n| !staging.join(n).exists())
        .unwrap_or_default();
    fs::rename(dir.join("mod"), staging.join(&name)).at(&dir)?;
    fs::remove_dir_all(&dir).at(&dir)?;
    Ok((name, item))
}

/// Deletes a trash entry for good.
pub fn purge(trash: &Path, id: &str) -> Result<()> {
    let dir = trash.join(id);
    fs::remove_dir_all(&dir).at(&dir)
}

/// Keeps the newest `keep` entries.
pub fn prune(trash: &Path, keep: usize) -> Result<usize> {
    let old: Vec<TrashItem> = list(trash).into_iter().skip(keep).collect();
    old.iter().try_for_each(|item| purge(trash, &item.id))?;
    Ok(old.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mod_engine::model::ModKind;

    #[test]
    fn deleted_mods_can_be_restored() {
        let tmp = tempfile::tempdir().unwrap();
        let (staging, trash) = (tmp.path().join("staging"), tmp.path().join("trash"));
        fs::create_dir_all(staging.join("Camera")).unwrap();
        fs::write(staging.join("Camera/cam.asi"), "x").unwrap();
        let entry = ModEntry {
            name: "Camera".into(),
            kind: ModKind::Asi,
            enabled: true,
            file_count: 1,
            paz_files: 0,
            installed_at: 1,
            source: "nexus:x".into(),
            nexus: Some(NexusRef {
                mod_id: 4,
                file_id: 40,
                version: "2".into(),
            }),
            variant: Some("Wide".into()),
        };
        let item = move_to_trash(&staging, &trash, &entry, 100).unwrap();
        assert!(!staging.join("Camera").exists());
        assert_eq!(
            (
                item.nexus.as_ref().map(|n| n.file_id),
                item.variant.as_deref()
            ),
            (Some(40), Some("Wide"))
        );
        fs::create_dir_all(staging.join("Camera")).unwrap();
        let (name, restored) = restore(&trash, &staging, &item.id).unwrap();
        assert_eq!(
            (name.as_str(), restored.source.as_str()),
            ("Camera (2)", "nexus:x")
        );
        assert!(staging.join("Camera (2)/cam.asi").is_file());
        assert!(list(&trash).is_empty());
        (0..5).for_each(|i| {
            fs::create_dir_all(staging.join(format!("M{i}"))).unwrap();
            move_to_trash(
                &staging,
                &trash,
                &ModEntry {
                    name: format!("M{i}"),
                    ..entry.clone()
                },
                i,
            )
            .unwrap();
        });
        assert_eq!(prune(&trash, 2).unwrap(), 3);
        assert_eq!(
            list(&trash)
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>(),
            ["M4", "M3"]
        );
    }
}
