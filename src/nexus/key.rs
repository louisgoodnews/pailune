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
        key => OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .and_then(|mut file| file.write_all(key.as_bytes()))
            .at(path),
    }
}
