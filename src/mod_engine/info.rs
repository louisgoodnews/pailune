//! Mod details: file listing, sizes and readme discovery.

use super::model::{ModDetails, StagedFile};
use super::plan::{Placement, deployable_files};
use crate::error::{IoContext, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Largest readme shown in the viewer.
const README_LIMIT: u64 = 512 * 1024;

/// Whether a file looks like documentation worth showing.
fn is_readme(rel: &Path) -> bool {
    let name = rel
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ext = rel
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let doc_name = ["readme", "changelog", "install", "license", "credits"]
        .iter()
        .any(|k| name.contains(k));
    matches!(ext.as_str(), "txt" | "md" | "nfo") || (doc_name && ext.is_empty())
}

/// Collects files, sizes and readmes of the mod in `mod_root`.
/// For multi-variant mods, files of the other variants are listed as not
/// deployed.
pub fn details(mod_root: &Path, variant: Option<&str>) -> Result<ModDetails> {
    let (installer, installer_error) = match super::plan::installer(mod_root) {
        Ok(installer) => (installer, None),
        Err(err) => (None, Some(err)),
    };
    let deployed: HashMap<PathBuf, Placement> = deployable_files(mod_root, variant)?
        .into_iter()
        .map(|f| (f.source, f.place))
        .collect();
    let files = WalkDir::new(mod_root)
        .sort_by_file_name()
        .into_iter()
        .filter(|e| e.as_ref().map_or(true, |e| e.file_type().is_file()))
        .map(|entry| {
            let entry = entry?;
            let rel = entry
                .path()
                .strip_prefix(mod_root)
                .unwrap_or(entry.path())
                .to_path_buf();
            let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let placement = deployed
                .get(entry.path())
                .copied()
                .unwrap_or(Placement::Skip);
            Ok(StagedFile {
                rel,
                bytes,
                placement,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ModDetails {
        total_bytes: files.iter().map(|f| f.bytes).sum(),
        readmes: files
            .iter()
            .filter(|f| is_readme(&f.rel))
            .map(|f| f.rel.clone())
            .collect(),
        files,
        variants: match (&installer, &installer_error) {
            (None, None) => super::variants::variants(mod_root),
            _ => Vec::new(),
        },
        installer,
        installer_error,
    })
}

/// Reads a text file for display, truncated and decoded lossily.
pub fn read_text(path: &Path) -> Result<String> {
    let mut buf = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(README_LIMIT).read_to_end(&mut buf))
        .at(path)?;
    Ok(String::from_utf8_lossy(&buf).replace('\r', ""))
}
