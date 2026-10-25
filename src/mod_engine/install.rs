//! Importing mods (folders, `.zip` archives, loose files) into staging.

use super::archive::{self, ArchiveKind};
use super::plan::is_archive_dir;
use crate::error::{IoContext, PyluneError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// What kind of source the user picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Directory,
    Archive(ArchiveKind),
    LooseFile,
}

/// Copies or extracts `source` into a new folder under `staging`.
///
/// Returns the mod name (the staging sub-directory). The source is never
/// modified or moved.
pub fn import(source: &Path, staging: &Path) -> Result<String> {
    import_as(source, staging, &mod_name(source)?, false)
}

/// A name unique to this process and moment, for temporary entries.
pub fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{n}", std::process::id())
}

/// Like [`import`] but with an explicit mod name.
///
/// Every call stages into its own hidden work directory, so concurrent
/// imports never touch each other's files. Without `replace` the final step
/// fails if the name was taken meanwhile; with `replace` the existing mod is
/// swapped out only after the new contents were staged successfully.
pub fn import_as(source: &Path, staging: &Path, name: &str, replace: bool) -> Result<String> {
    let dest = staging.join(name);
    let taken = || PyluneError::Invalid(format!("a mod named '{name}' is already staged"));
    match (dest.exists(), replace) {
        (true, false) => return Err(taken()),
        (false, true) => {
            return Err(PyluneError::Invalid(format!(
                "'{name}' is no longer staged"
            )));
        }
        _ => {}
    }
    if source.starts_with(staging) || staging.starts_with(source) {
        return Err(PyluneError::Invalid(
            "cannot import from inside Pylune's staging folder".into(),
        ));
    }
    let work = staging.join(format!(".pylune-incoming-{}", unique_suffix()));
    fs::create_dir_all(&work).at(&work)?;
    let outcome = match source_kind(source) {
        SourceKind::Directory => copy_tree(source, &work),
        SourceKind::Archive(kind) => archive::extract(source, kind, &work),
        SourceKind::LooseFile => copy_file(source, &work),
    }
    .and_then(|()| hoist_wrapper_dirs(&work))
    .and_then(|()| match replace {
        true => swap_into_place(&work, &dest, staging),
        false => claim_and_move(&work, &dest).map_err(|_| taken()),
    });
    // Leave no half-imported mod behind.
    outcome.map(|()| name.to_string()).inspect_err(|_| {
        let _ = fs::remove_dir_all(&work);
    })
}

/// Moves `work` to a free `dest`. `create_dir` is atomic, so exactly one of
/// several concurrent imports claims a name; renaming a directory onto the
/// claimed empty directory then succeeds.
fn claim_and_move(work: &Path, dest: &Path) -> Result<()> {
    fs::create_dir(dest).at(dest)?;
    fs::rename(work, dest).at(dest)
}

/// Replaces `dest` with `work` using renames only, so the mod's folder is
/// missing for just an instant; the old contents are deleted afterwards.
fn swap_into_place(work: &Path, dest: &Path, staging: &Path) -> Result<()> {
    let trash = staging.join(format!(".pylune-trash-{}", unique_suffix()));
    fs::rename(dest, &trash).at(dest)?;
    fs::rename(work, dest).at(dest)?;
    fs::remove_dir_all(&trash).at(&trash)
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

/// Classifies what the user picked by type and extension.
fn source_kind(source: &Path) -> SourceKind {
    match (source.is_dir(), archive::kind_of(source)) {
        (true, _) => SourceKind::Directory,
        (false, Some(kind)) => SourceKind::Archive(kind),
        (false, None) => SourceKind::LooseFile,
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

/// Copies a single file into the staging folder.
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
