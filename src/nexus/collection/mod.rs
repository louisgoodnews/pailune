//! Nexus collections: curated mod lists treated as a parent with sub-mods.

use serde_json::Value;
use std::collections::BTreeSet;

/// One mod file listed by a collection revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Curator order (0-based); used as load order.
    pub position: usize,
    pub mod_id: u64,
    pub file_id: u64,
    pub optional: bool,
    /// Whether the user chose to install it (required members always are).
    pub selected: bool,
    /// "Mod name — file name", for display.
    pub title: String,
    pub size: u64,
    /// Staged mod name once installed.
    pub mod_name: Option<String>,
}

/// A collection revision (fetched from Nexus or as installed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    pub slug: String,
    pub name: String,
    pub revision: u32,
    pub members: Vec<Member>,
}

/// What updating an installed collection to a new revision does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdatePlan {
    /// Members to download; `Some(name)` replaces that staged mod.
    pub download: Vec<(Member, Option<String>)>,
    /// Staged mods that left the collection.
    pub remove: Vec<String>,
}

/// Parses a `collectionRevision` GraphQL response.
pub fn from_graphql(slug: &str, data: &Value) -> Option<Collection> {
    let rev = data.get("data")?.get("collectionRevision")?;
    let members = rev
        .get("modFiles")?
        .as_array()?
        .iter()
        .enumerate()
        .filter_map(|(position, m)| {
            let file = m.get("file")?;
            let optional = m.get("optional").and_then(Value::as_bool).unwrap_or(false);
            let mod_name = file
                .get("mod")
                .and_then(|x| x.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("?");
            let file_name = file.get("name").and_then(Value::as_str).unwrap_or("?");
            Some(Member {
                position,
                mod_id: file.get("modId")?.as_u64()?,
                file_id: m.get("fileId")?.as_u64()?,
                optional,
                selected: !optional,
                title: format!("{mod_name} — {file_name}"),
                size: file
                    .get("sizeInBytes")
                    .and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or(v.as_u64()))
                    .unwrap_or(0),
                mod_name: None,
            })
        })
        .collect();
    Some(Collection {
        slug: slug.to_string(),
        name: rev.get("collection")?.get("name")?.as_str()?.to_string(),
        revision: rev.get("revisionNumber")?.as_u64()? as u32,
        members,
    })
}

/// Accepts a slug or a collection URL (`…/collections/<slug>[/…]`).
pub fn parse_slug(input: &str) -> Option<String> {
    let trimmed = input.trim().trim_end_matches('/');
    let tail = trimmed.split("/collections/").nth(1).unwrap_or(trimmed);
    let slug = tail.split(['/', '?', '#']).next()?;
    (!slug.is_empty() && slug.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| slug.to_lowercase())
}

/// Staged mods of `collection` that no other installed collection uses.
pub fn exclusive_mods(collection: &Collection, others: &[Collection]) -> Vec<String> {
    let shared: BTreeSet<&String> = others
        .iter()
        .filter(|c| c.slug != collection.slug)
        .flat_map(|c| c.members.iter().filter_map(|m| m.mod_name.as_ref()))
        .collect();
    collection
        .members
        .iter()
        .filter_map(|m| m.mod_name.clone())
        .filter(|n| !shared.contains(n))
        .collect()
}

/// Compares an installed collection with a newer revision (by mod id):
/// unchanged files are kept, changed files replace the staged mod, new
/// required mods are added, and dropped mods are removed.
pub fn plan_update(
    installed: &Collection,
    latest: &Collection,
    others: &[Collection],
) -> UpdatePlan {
    let previous = |mod_id| installed.members.iter().find(|m| m.mod_id == mod_id);
    let download = latest
        .members
        .iter()
        .filter_map(|m| match previous(m.mod_id) {
            Some(old) if old.file_id == m.file_id && old.mod_name.is_some() => None,
            Some(old) if old.selected => Some((
                Member {
                    selected: true,
                    ..m.clone()
                },
                old.mod_name.clone(),
            )),
            Some(_) => None,
            None => m.selected.then(|| (m.clone(), None)),
        })
        .collect();
    let kept: BTreeSet<u64> = latest.members.iter().map(|m| m.mod_id).collect();
    let dropped = Collection {
        members: installed
            .members
            .iter()
            .filter(|m| !kept.contains(&m.mod_id))
            .cloned()
            .collect(),
        ..installed.clone()
    };
    UpdatePlan {
        download,
        remove: exclusive_mods(&dropped, others),
    }
}

#[cfg(test)]
mod tests;
