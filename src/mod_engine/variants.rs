//! Multi-variant mods: one archive with several alternative option folders
//! (e.g. `Character Creator/Human Female/0009/…` and `…/Dwarf Female/0009/…`).
//! Only the chosen variant is deployed; shared files at the root always are.
//! Without option folders, overlapping JSON files are the options instead
//! ([`super::alternatives`]); a variant is then the file's relative path.

use super::plan::is_archive_dir;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Folders that are part of the game layout, never variant names.
fn is_layout_dir(name: &str) -> bool {
    let lower = name.to_lowercase();
    is_archive_dir(&lower) || matches!(lower.as_str(), "bin64" | "meta" | "files")
}

/// Whether a folder contains game-layout content (`NNNN/`, `files/`, `bin64/`).
fn has_layout(dir: &Path) -> bool {
    WalkDir::new(dir)
        .min_depth(1)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|e| e.file_type().is_dir() && is_layout_dir(&e.file_name().to_string_lossy()))
}

/// The options of a staged mod with their display names: variant folders
/// (sorted), else alternative JSON files (titled by their `modinfo`); empty
/// unless there are at least two. A single layout folder is no choice but a
/// wrapper (see [`layout_folders`]).
pub fn titled(mod_root: &Path) -> Vec<(String, String)> {
    match layout_folders(mod_root).as_slice() {
        [] | [_] => super::alternatives::alternatives(mod_root)
            .into_iter()
            .map(|(path, title)| {
                let shown = match title.is_empty() {
                    true => path.clone(),
                    false => title,
                };
                (path, shown)
            })
            .collect(),
        found => found.iter().map(|f| (f.clone(), f.clone())).collect(),
    }
}

/// The options of a staged mod (see [`titled`]).
pub fn variants(mod_root: &Path) -> Vec<String> {
    titled(mod_root).into_iter().map(|(v, _)| v).collect()
}

/// Folders at the mod root that hold game-layout content (`NNNN/`, `files/`,
/// `bin64/`), sorted. Several are alternative options; a single one is a
/// wrapper (`Mod/Mod Enhanced/0009/…` next to the mod's plugin or JSON) that
/// is deployed as if its content sat at the root.
pub fn layout_folders(mod_root: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(mod_root)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|name| !name.starts_with('.') && !is_layout_dir(name))
                .filter(|name| has_layout(&mod_root.join(name)))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// The variant to deploy: the chosen one if it still exists, else the first.
pub fn effective<'a>(variants: &'a [String], chosen: Option<&str>) -> Option<&'a String> {
    variants
        .iter()
        .find(|v| Some(v.as_str()) == chosen)
        .or_else(|| variants.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_alternative_option_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        [
            "Human Female/0009/character/a.xml",
            "Dwarf Female/0009/character/a.xml",
            "docs/readme.txt",
            "0012/ui/x.css",
        ]
        .iter()
        .for_each(|p| {
            fs::create_dir_all(root.join(p).parent().unwrap()).unwrap();
            fs::write(root.join(p), "x").unwrap();
        });
        let found = variants(root);
        assert_eq!(found, ["Dwarf Female", "Human Female"]);
        assert_eq!(
            effective(&found, Some("Human Female")).map(String::as_str),
            Some("Human Female")
        );
        assert_eq!(
            effective(&found, Some("gone")).map(String::as_str),
            Some("Dwarf Female")
        );
        assert!(
            variants(&root.join("Human Female")).is_empty(),
            "a single layout is no variant set"
        );
    }

    #[test]
    fn a_single_layout_folder_is_a_wrapper() {
        use crate::mod_engine::plan::{Placement, deployable_files};
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        [
            "Plugin.asi",
            "Mod Enhanced/0009/character/a.pac",
            "readme.txt",
        ]
        .iter()
        .for_each(|p| {
            fs::create_dir_all(root.join(p).parent().unwrap()).unwrap();
            fs::write(root.join(p), "x").unwrap();
        });
        assert!(variants(root).is_empty(), "no choice to make");
        let files: Vec<(String, Placement)> = deployable_files(root, None)
            .unwrap()
            .into_iter()
            .map(|f| (f.rel.to_string_lossy().replace('\\', "/"), f.place))
            .collect();
        assert_eq!(
            files,
            [
                ("0009/character/a.pac".to_string(), Placement::Pack),
                ("Plugin.asi".to_string(), Placement::Bin64)
            ]
        );
    }
}
