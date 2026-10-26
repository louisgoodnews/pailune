//! Alternative JSON files: mods like *Fat Stacks* ship one JSON file per
//! choice (`…_2x.field.json`, `…_10x.field.json`) in one folder. Files that
//! change the same place of the same game file cannot all apply — the last
//! would silently win — so they are offered as options like variant
//! folders. Files that change different places stay independent.

use super::placement::{Placement, placement};
use crate::patch::{Anchor, JsonMod};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;
use walkdir::WalkDir;

/// One changed place: game file stem and record/offset/field.
type Place = (String, String);

/// What a JSON file reads as: its title and the places it changes.
#[derive(Clone)]
struct Summary {
    title: String,
    places: BTreeSet<Place>,
}

/// The game file without folders and extension (`iteminfo`), so old and
/// new extensions and short or full paths compare equal.
fn stem(game_file: &str) -> String {
    let name = game_file.rsplit(['/', '\\']).next().unwrap_or(game_file);
    name.split('.').next().unwrap_or(name).to_lowercase()
}

fn summarize(json: &JsonMod) -> Summary {
    let places = json.patches.iter().flat_map(|p| {
        let file = stem(&p.game_file);
        let bytes = p.changes.iter().map(|c| match &c.anchor {
            Anchor::Offset(o) => format!("@{o}"),
            Anchor::Entry { name, rel_offset } => format!("{name}+{rel_offset}"),
        });
        let fields = p.fields.iter().map(|i| {
            let record = i
                .key
                .map_or(i.entry.clone().unwrap_or_default(), |k| k.to_string());
            let what = i.field.clone().or(i.old.clone()).unwrap_or_default();
            format!("{record}.{}", what.to_lowercase().replace('_', ""))
        });
        bytes.chain(fields).map(move |place| (file.clone(), place))
    });
    Summary {
        title: json.title.clone(),
        places: places.collect(),
    }
}

/// The summary of a JSON file, cached by path, size and modification time
/// (these files can be large and are looked at on every plan).
fn summary(path: &Path) -> Option<Summary> {
    type Cache = HashMap<PathBuf, (u64, SystemTime, Option<Summary>)>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let meta = std::fs::metadata(path).ok()?;
    let stamp = (meta.len(), meta.modified().ok()?);
    let mut cache = CACHE.get_or_init(Default::default).lock().ok()?;
    if let Some((len, time, cached)) = cache.get(path)
        && (*len, *time) == stamp
    {
        return cached.clone();
    }
    let parsed = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| crate::patch::parse::parse(&text).ok())
        .map(|json| summarize(&json));
    cache.insert(path.to_path_buf(), (stamp.0, stamp.1, parsed.clone()));
    parsed
}

/// Orders names with numbers by value: `2x` before `10x`.
pub fn natural_key(name: &str) -> Vec<(String, u64)> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut digits = String::new();
    for c in name.to_lowercase().chars().chain(std::iter::once('\0')) {
        match c.is_ascii_digit() {
            true => digits.push(c),
            false => {
                if !digits.is_empty() {
                    let value = digits.parse().unwrap_or(u64::MAX);
                    parts.push((std::mem::take(&mut text), value));
                    digits.clear();
                }
                text.push(c);
            }
        }
    }
    parts.push((text, 0));
    parts
}

/// The alternative JSON files of a staged mod (paths relative to its root,
/// `/`-separated, in natural order) with their titles. Empty unless one
/// set of at least two files overlaps; with several such sets nothing is
/// offered (the choice would be ambiguous).
pub fn alternatives(mod_root: &Path) -> Vec<(String, String)> {
    let files: Vec<(String, Summary)> = WalkDir::new(mod_root)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let rel = e.path().strip_prefix(mod_root).ok()?.to_path_buf();
            (placement(&rel) == Placement::JsonPatch).then_some(())?;
            let key = rel.to_string_lossy().replace('\\', "/");
            summary(e.path()).map(|s| (key, s))
        })
        .collect();
    // Connected sets of files that share a changed place.
    let mut group: Vec<usize> = (0..files.len()).collect();
    fn root(group: &mut [usize], i: usize) -> usize {
        match group[i] == i {
            true => i,
            false => {
                let r = root(group, group[i]);
                group[i] = r;
                r
            }
        }
    }
    for a in 0..files.len() {
        for b in a + 1..files.len() {
            if !files[a].1.places.is_disjoint(&files[b].1.places) {
                let (ra, rb) = (root(&mut group, a), root(&mut group, b));
                group[ra] = rb;
            }
        }
    }
    let mut sets: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    (0..files.len()).for_each(|i| sets.entry(root(&mut group, i)).or_default().push(i));
    let mut overlapping = sets.into_values().filter(|s| s.len() > 1);
    let (Some(set), None) = (overlapping.next(), overlapping.next()) else {
        return Vec::new();
    };
    let mut found: Vec<(String, String)> = set
        .into_iter()
        .map(|i| (files[i].0.clone(), files[i].1.title.clone()))
        .collect();
    found.sort_by_key(|(path, _)| natural_key(path));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stacks(factor: u64) -> String {
        format!(
            r#"{{"modinfo": {{"title": "Stacks {factor}X"}}, "format": 3, "target": "iteminfo.pabgb",
              "intents": [{{"entry": "Arrow", "key": 50001, "field": "max_stack_count", "new": {}}}]}}"#,
            100 * factor
        )
    }

    #[test]
    fn overlapping_json_files_are_alternatives_in_natural_order() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        [10, 2, 5].iter().for_each(|f| {
            std::fs::write(root.join(format!("stacks_{f}x.field.json")), stacks(*f)).unwrap();
        });
        // Changes another place: deployed alongside whichever is chosen.
        let other = r#"{"patches": [{"game_file": "skill.pabgb", "changes": [
            {"offset": 4, "original": "00", "patched": "01"}]}]}"#;
        std::fs::write(root.join("extra.json"), other).unwrap();
        let found = alternatives(root);
        let names: Vec<_> = found
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                ("stacks_2x.field.json", "Stacks 2X"),
                ("stacks_5x.field.json", "Stacks 5X"),
                ("stacks_10x.field.json", "Stacks 10X")
            ]
        );
        std::fs::remove_file(root.join("stacks_5x.field.json")).unwrap();
        std::fs::remove_file(root.join("stacks_10x.field.json")).unwrap();
        assert!(alternatives(root).is_empty(), "one file is no choice");
    }
}
