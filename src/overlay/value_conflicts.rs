//! Conflicts inside game files: two JSON patches that change the same
//! bytes. Linked and packed files conflict per file; JSON patches only
//! when their changes overlap, so every enabled patch is resolved against
//! the vanilla data (format-3 fields included) and the byte ranges compared.
//! Only the game's archives are read.

use super::patches::{read_all, resolve, vanilla_bytes, wanted};
use super::table_paths::header_of;
use crate::error::Result;
use crate::patch::apply::spans;
use crate::paz::vanilla;
use crate::steam::SteamInfo;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Two mods changing the same values of one game file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlap {
    pub vpath: String,
    /// Earlier in the load order: overridden.
    pub earlier: String,
    /// Later in the load order: wins.
    pub later: String,
    /// Records (or change labels) both change.
    pub values: BTreeSet<String>,
}

/// A game file as the patches see it: body, table index, records.
struct Target {
    body: Vec<u8>,
    header: Option<Vec<u8>>,
    records: Vec<crate::tables::Record>,
}

/// The record holding byte `at`, else the change's own label.
fn value_name(target: &Target, at: usize, label: &str) -> String {
    target
        .records
        .iter()
        .find(|r| r.start <= at && at < r.end)
        .map_or(label.to_string(), |r| r.name.clone())
}

/// Overlapping changes of the JSON patches `(mod, file)`, given in load
/// order. Nothing is read when fewer than two mods ship patches.
pub fn overlaps(info: &SteamInfo, patches: &[(String, PathBuf)]) -> Result<Vec<Overlap>> {
    let mods = read_all(patches);
    let authors: BTreeSet<&String> = mods
        .iter()
        .filter(|m| m.parsed.is_ok())
        .map(|m| &m.mod_name)
        .collect();
    if authors.len() < 2 {
        return Ok(Vec::new());
    }
    let base = vanilla::load_cached(
        &info.game_dir,
        &super::build::vanilla_papgt(info)?,
        &wanted(&mods),
    )?;
    let mut targets: BTreeMap<String, Target> = BTreeMap::new();
    let mut taken: BTreeMap<String, Vec<(usize, usize, String)>> = BTreeMap::new();
    let mut found: BTreeMap<(String, String, String), BTreeSet<String>> = BTreeMap::new();
    let parsed = mods
        .iter()
        .filter_map(|m| m.parsed.as_ref().ok().map(|p| (m, p)));
    for (m, json) in parsed {
        for p in &json.patches {
            let Ok(vpath) = resolve(&base, &p.game_file) else {
                continue;
            };
            if !targets.contains_key(&vpath) {
                let Ok(body) = vanilla_bytes(&info.game_dir, &base, &vpath) else {
                    continue;
                };
                let header =
                    header_of(&vpath).and_then(|h| vanilla_bytes(&info.game_dir, &base, &h).ok());
                let records = header
                    .as_deref()
                    .and_then(|h| crate::tables::records(h, &body).ok())
                    .unwrap_or_default();
                targets.insert(
                    vpath.clone(),
                    Target {
                        body,
                        header,
                        records,
                    },
                );
            }
            let target = &targets[&vpath];
            let header = target.header.as_deref();
            let (patch, _) = crate::tables::format3::with_fields(p, &vpath, header, &target.body);
            let used = taken.entry(vpath.clone()).or_default();
            for (start, end, label) in spans(&target.body, &patch, header) {
                used.iter()
                    .filter(|(s, e, other)| *other != m.mod_name && start < *e && *s < end)
                    .for_each(|(_, _, other)| {
                        found
                            .entry((vpath.clone(), other.clone(), m.mod_name.clone()))
                            .or_default()
                            .insert(value_name(target, start, &label));
                    });
                used.push((start, end, m.mod_name.clone()));
            }
        }
    }
    Ok(found
        .into_iter()
        .map(|((vpath, earlier, later), values)| Overlap {
            vpath,
            earlier,
            later,
            values,
        })
        .collect())
}
