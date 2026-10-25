//! Importing mods (folders, `.zip` archives, loose files) into staging.

use super::plan::is_archive_dir;
use crate::error::{IoContext, PyluneError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// What kind of source the user picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Directory,
    Zip,
    LooseFile,
    Unsupported,
}

/// Copies or extracts `source` into a new folder under `staging`.
///
/// Returns the mod name (the staging sub-directory). The source is never
/// modified or moved.
pub fn import(source: &Path, staging: &Path) -> Result<String> {
    import_as(source, staging, &mod_name(source)?, false)
}

/// Like [`import`] but with an explicit mod name.
///
/// With `replace`, an existing mod of that name is swapped out only after the
/// new contents were staged successfully (used for Nexus updates).
pub fn import_as(source: &Path, staging: &Path, name: &str, replace: bool) -> Result<String> {
    let dest = staging.join(name);
    if dest.exists() && !replace {
        return Err(PyluneError::Invalid(format!(
            "a mod named '{name}' is already staged"
        )));
    }
    let work = staging.join(format!(".pylune-incoming-{name}"));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).at(&work)?;
    let outcome = match source_kind(source) {
        SourceKind::Directory => copy_tree(source, &work),
        SourceKind::Zip => extract_zip(source, &work),
        SourceKind::LooseFile => copy_file(source, &work),
        SourceKind::Unsupported => Err(PyluneError::Invalid(format!(
            "{} is a 7z/rar archive — extract it and import the folder",
            source.display()
        ))),
    }
    .and_then(|()| hoist_wrapper_dirs(&work))
    .and_then(|()| swap_into_place(&work, &dest));
    // Leave no half-imported mod behind.
    outcome.map(|()| name.to_string()).inspect_err(|_| {
        let _ = fs::remove_dir_all(&work);
    })
}

/// Replaces `dest` with the freshly staged `work` directory.
fn swap_into_place(work: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        fs::remove_dir_all(dest).at(dest)?;
    }
    fs::rename(work, dest).at(dest)
}

/// Makes an arbitrary title safe to use as a staging directory name.
pub fn sanitize_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    match cleaned.trim().trim_start_matches('.') {
        "" => "Unnamed mod".to_string(),
        name => name.to_string(),
    }
}

fn source_kind(source: &Path) -> SourceKind {
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match (source.is_dir(), ext.as_str()) {
        (true, _) => SourceKind::Directory,
        (false, "zip") => SourceKind::Zip,
        (false, "7z" | "rar") => SourceKind::Unsupported,
        (false, _) => SourceKind::LooseFile,
    }
}

/// Derives a mod name from the file or folder name (without archive suffix).
fn mod_name(source: &Path) -> Result<String> {
    let raw = match source_kind(source) {
        SourceKind::Directory => source.file_name(),
        _ => source.file_stem(),
    };
    raw.map(|n| n.to_string_lossy().trim().to_string())
        .filter(|n| !n.is_empty() && !n.starts_with('.'))
        .ok_or_else(|| PyluneError::Invalid(format!("cannot name mod from {}", source.display())))
}

fn copy_file(source: &Path, dest_dir: &Path) -> Result<()> {
    let name = source
        .file_name()
        .ok_or_else(|| PyluneError::Invalid("source has no file name".into()))?;
    fs::copy(source, dest_dir.join(name)).at(source).map(|_| ())
}

/// Recursively copies a directory, following no symlinks.
fn copy_tree(source: &Path, dest: &Path) -> Result<()> {
    WalkDir::new(source)
        .min_depth(1)
        .into_iter()
        .try_for_each(|entry| {
            let entry = entry?;
            let rel = entry.path().strip_prefix(source).unwrap_or(entry.path());
            let target = dest.join(rel);
            match entry.file_type() {
                t if t.is_dir() => fs::create_dir_all(&target).at(&target),
                t if t.is_file() => fs::copy(entry.path(), &target).at(&target).map(|_| ()),
                _ => Ok(()),
            }
        })
}

/// Extracts a zip archive; entries escaping `dest` are rejected by `zip`.
fn extract_zip(source: &Path, dest: &Path) -> Result<()> {
    let file = fs::File::open(source).at(source)?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(dest)?;
    Ok(())
}

/// Returns the single child directory if `dir` contains nothing else.
fn sole_child_dir(dir: &Path) -> Result<Option<PathBuf>> {
    let children = fs::read_dir(dir)
        .at(dir)?
        .map(|e| e.map(|e| e.path()).at(dir))
        .collect::<Result<Vec<_>>>()?;
    Ok(match children.as_slice() {
        [only] if only.is_dir() => Some(only.clone()),
        _ => None,
    })
}

/// Whether a directory name is part of the game layout and must stay.
fn is_game_dir(path: &Path) -> bool {
    path.file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .is_some_and(|n| n == "bin64" || n == "meta" || is_archive_dir(&n))
}

/// Strips redundant wrapper folders (`MyMod/MyMod-1.2/...`) common in archives.
fn hoist_wrapper_dirs(dest: &Path) -> Result<()> {
    match sole_child_dir(dest)? {
        Some(child) if !is_game_dir(&child) => {
            let tmp = dest.join(".pylune-hoist");
            fs::rename(&child, &tmp).at(&child)?;
            fs::read_dir(&tmp).at(&tmp)?.try_for_each(|entry| {
                let from = entry.at(&tmp)?.path();
                let to = dest.join(from.file_name().unwrap_or_default());
                fs::rename(&from, &to).at(&from)
            })?;
            fs::remove_dir(&tmp).at(&tmp)?;
            hoist_wrapper_dirs(dest)
        }
        _ => Ok(()),
    }
}
