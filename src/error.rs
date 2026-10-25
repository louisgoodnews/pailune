//! Crate-wide error type and helpers for attaching paths to I/O failures.

use std::io;
use std::path::{Path, PathBuf};

/// Every failure Pylune can surface to the user.
#[derive(Debug, thiserror::Error)]
pub enum PyluneError {
    /// A filesystem operation failed on a specific path.
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    /// A JSON document could not be read or written.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// A mod archive could not be extracted.
    #[error("archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// Directory traversal failed.
    #[error("walk error: {0}")]
    Walk(#[from] walkdir::Error),
    /// A precondition was not met (bad input, missing game, ...).
    #[error("{0}")]
    Invalid(String),
}

/// Shorthand result type used throughout Pylune.
pub type Result<T> = std::result::Result<T, PyluneError>;

/// Attaches the offending path to an [`io::Result`].
pub trait IoContext<T> {
    /// Converts the error into [`PyluneError::Io`] carrying `path`.
    fn at(self, path: &Path) -> Result<T>;
}

impl<T> IoContext<T> for io::Result<T> {
    fn at(self, path: &Path) -> Result<T> {
        self.map_err(|source| PyluneError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}
