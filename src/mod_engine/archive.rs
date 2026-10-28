//! Extracting `.zip`, `.7z` and `.rar` archives into a staging directory.
//!
//! Every entry path is checked before writing so archives cannot escape the
//! destination (`../`, absolute paths).

use crate::error::{PailuneError, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Supported archive formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    SevenZip,
    Rar,
}

/// Detects the archive format from the file extension.
pub fn kind_of(path: &Path) -> Option<ArchiveKind> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "zip" => Some(ArchiveKind::Zip),
        "7z" => Some(ArchiveKind::SevenZip),
        "rar" => Some(ArchiveKind::Rar),
        _ => None,
    }
}

/// Whether an entry path stays inside the extraction root.
pub fn is_safe_entry(entry: &Path) -> bool {
    entry
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// Rejects an unsafe entry with a descriptive error.
fn guard(entry: &Path) -> Result<()> {
    match is_safe_entry(entry) {
        true => Ok(()),
        false => Err(PailuneError::Archive(format!(
            "unsafe path in archive: {}",
            entry.display()
        ))),
    }
}

/// Extracts `archive` into the existing directory `dest`.
pub fn extract(archive: &Path, kind: ArchiveKind, dest: &Path) -> Result<()> {
    match kind {
        ArchiveKind::Zip => extract_zip(archive, dest),
        ArchiveKind::SevenZip => extract_7z(archive, dest),
        ArchiveKind::Rar => extract_rar(archive, dest),
    }?;
    sanitize_permissions(dest)
}

/// Normalises file and directory permissions in `dest` (equivalent to `u+rwX`)
/// so archives created with broken attribute bits (missing write or traversal bits)
/// can be read, walked, and cleaned up by the user.
pub fn sanitize_permissions(dest: &Path) -> Result<()> {
    sanitize_recursive(dest)
}

fn sanitize_recursive(dir: &Path) -> Result<()> {
    ensure_user_perms(dir, true);
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries.filter_map(std::result::Result::ok) {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            sanitize_recursive(&path)?;
        } else {
            ensure_user_perms(&path, false);
        }
    }
    Ok(())
}

fn ensure_user_perms(path: &Path, is_dir: bool) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::symlink_metadata(path) {
            let current = meta.permissions().mode();
            let current_perms = current & 0o7777;
            // Equivalent to u+rwX: directories or files with any execute bit get u+rwx (0o700);
            // other files get u+rw (0o600).
            let desired = if is_dir || (current_perms & 0o111 != 0) {
                current_perms | 0o700
            } else {
                current_perms | 0o600
            };
            if current_perms != desired {
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(desired));
            }
        }
    }
    #[cfg(not(unix))]
    {
        if let Ok(meta) = fs::symlink_metadata(path) {
            let mut perms = meta.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = fs::set_permissions(path, perms);
            }
        }
    }
}

/// `zip` itself refuses entries escaping `dest`.
fn extract_zip(archive: &Path, dest: &Path) -> Result<()> {
    let file = fs::File::open(archive).map_err(|e| PailuneError::Archive(e.to_string()))?;
    zip::ZipArchive::new(file)?.extract(dest)?;
    Ok(())
}

/// Extracts a 7z archive, rejecting entries outside `dest`.
fn extract_7z(archive: &Path, dest: &Path) -> Result<()> {
    let root: PathBuf = dest.to_path_buf();
    sevenz_rust2::decompress_file_with_extract_fn(archive, dest, |entry, reader, target| {
        let inside = target.strip_prefix(&root).is_ok_and(is_safe_entry)
            && is_safe_entry(Path::new(entry.name()));
        match inside {
            true => sevenz_rust2::default_entry_extract_fn(entry, reader, target),
            false => Err(sevenz_rust2::Error::Other(
                format!("unsafe path in archive: {}", entry.name()).into(),
            )),
        }
    })?;
    Ok(())
}

/// Walks the RAR headers, extracting files and skipping everything else.
fn extract_rar(archive: &Path, dest: &Path) -> Result<()> {
    let mut cursor = unrar::Archive::new(archive).open_for_processing()?;
    while let Some(header) = cursor.read_header()? {
        let entry = header.entry();
        guard(&entry.filename)?;
        cursor = match entry.is_file() {
            true => header.extract_with_base(dest)?,
            false => header.skip()?,
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_kinds_and_unsafe_paths() {
        assert_eq!(kind_of(Path::new("a/Mod.7Z")), Some(ArchiveKind::SevenZip));
        assert_eq!(kind_of(Path::new("m.rar")), Some(ArchiveKind::Rar));
        assert_eq!(kind_of(Path::new("m.asi")), None);
        assert!(is_safe_entry(Path::new("a/./b.txt")));
        assert!(!is_safe_entry(Path::new("../evil")));
        assert!(!is_safe_entry(Path::new("/etc/passwd")));
    }

    #[test]
    fn round_trips_a_7z_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(src.join("0036")).unwrap();
        fs::write(src.join("0036/0.paz"), "data").unwrap();
        let archive = tmp.path().join("mod.7z");
        sevenz_rust2::compress_to_path(&src, &archive).unwrap();
        let dest = tmp.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        extract(&archive, ArchiveKind::SevenZip, &dest).unwrap();
        assert_eq!(fs::read_to_string(dest.join("0036/0.paz")).unwrap(), "data");
    }

    #[test]
    #[cfg(unix)]
    fn sanitizes_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let file = sub.join("test.txt");
        fs::write(&file, "hello").unwrap();
        let exe = sub.join("script.sh");
        fs::write(&exe, "#!/bin/sh").unwrap();

        // Simulate broken permissions from archive headers
        fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o555)).unwrap();
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o555)).unwrap();

        sanitize_permissions(tmp.path()).unwrap();

        let sub_mode = fs::symlink_metadata(&sub).unwrap().permissions().mode() & 0o7777;
        let file_mode = fs::symlink_metadata(&file).unwrap().permissions().mode() & 0o7777;
        let exe_mode = fs::symlink_metadata(&exe).unwrap().permissions().mode() & 0o7777;

        // Subdirectory should have user rwx
        assert_eq!(sub_mode & 0o700, 0o700);
        // Regular file should have user rw
        assert_eq!(file_mode & 0o600, 0o600);
        // Executable file should have user rwx (keeps execute)
        assert_eq!(exe_mode & 0o700, 0o700);

        // Verification that directory can be walked and file read
        assert_eq!(fs::read_to_string(&file).unwrap(), "hello");
    }
}

#[cfg(test)]
mod rar_tests {
    use super::*;

    /// `tests/fixtures/sample.rar` (from the MIT/Apache `unrar` crate) holds `.gitignore`.
    #[test]
    fn extracts_a_rar_archive() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.rar");
        let dest = tempfile::tempdir().unwrap();
        extract(&archive, ArchiveKind::Rar, dest.path()).unwrap();
        assert!(dest.path().join(".gitignore").is_file());
    }
}
