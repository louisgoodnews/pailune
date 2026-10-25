//! Storing the Nexus API key in a private (`0600`) file.

use crate::error::{IoContext, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Reads the stored API key, if any.
pub fn load(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
}

/// Writes the API key readable only by the current user; empty removes it.
pub fn save(path: &Path, key: &str) -> Result<()> {
    match key.trim() {
        "" => fs::remove_file(path)
            .or_else(|e| match e.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(e),
            })
            .at(path),
        // Written to a private temp file and renamed, so the key is never
        // readable by others — even if an older key file had looser mode.
        key => {
            let tmp = path.with_extension("tmp");
            let _ = fs::remove_file(&tmp);
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp)
                .and_then(|mut file| file.write_all(key.as_bytes()))
                .and_then(|()| fs::rename(&tmp, path))
                .at(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn key_file_is_private_even_over_a_loose_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nexus_api_key");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        save(&path, " secret ").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(load(&path).as_deref(), Some("secret"));
        save(&path, "").unwrap();
        assert_eq!(load(&path), None);
    }
}
