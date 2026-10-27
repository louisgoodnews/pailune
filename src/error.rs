//! Crate-wide error type and helpers for attaching paths to I/O failures.

use std::io;
use std::path::{Path, PathBuf};

/// Every failure Pailune can surface to the user.
#[derive(Debug, thiserror::Error)]
pub enum PailuneError {
    /// A filesystem operation failed on a specific path.
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    /// A JSON document could not be read or written.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// The configuration file is malformed.
    #[error("config.toml: {0}")]
    TomlRead(#[from] toml::de::Error),
    /// The configuration could not be serialised.
    #[error("config.toml: {0}")]
    TomlWrite(#[from] toml::ser::Error),
    /// The SQLite database failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    /// A mod archive could not be extracted.
    #[error("archive error: {0}")]
    Archive(String),
    /// Directory traversal failed.
    #[error("walk error: {0}")]
    Walk(#[from] walkdir::Error),
    /// An HTTP request to Nexus Mods failed.
    #[error("network error: {0}")]
    Http(#[from] ureq::Error),
    /// A precondition was not met (bad input, missing game, ...).
    #[error("{0}")]
    Invalid(String),
}

impl From<zip::result::ZipError> for PailuneError {
    fn from(err: zip::result::ZipError) -> Self {
        Self::Archive(err.to_string())
    }
}

impl From<sevenz_rust2::Error> for PailuneError {
    fn from(err: sevenz_rust2::Error) -> Self {
        Self::Archive(err.to_string())
    }
}

impl From<unrar::error::UnrarError> for PailuneError {
    fn from(err: unrar::error::UnrarError) -> Self {
        Self::Archive(err.to_string())
    }
}

/// Shorthand result type used throughout Pailune.
pub type Result<T> = std::result::Result<T, PailuneError>;

/// Attaches the offending path to an [`io::Result`].
pub trait IoContext<T> {
    /// Converts the error into [`PailuneError::Io`] carrying `path`.
    fn at(self, path: &Path) -> Result<T>;
}

impl<T> IoContext<T> for io::Result<T> {
    fn at(self, path: &Path) -> Result<T> {
        self.map_err(|source| PailuneError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}
