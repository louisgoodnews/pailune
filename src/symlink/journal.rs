//! Write-ahead persistence of manifest changes.
//!
//! Deploy records every link *before* it touches the filesystem, and undeploy
//! forgets a link only *after* reverting it. Whenever the process dies, the
//! persisted manifest therefore covers every change that may have happened,
//! and the next revert stays exact.

use super::LinkRecord;
use crate::error::Result;
use std::path::Path;

/// Receives manifest changes as they happen.
pub trait Journal {
    /// A link is about to be created (its original may be moved to `.bak`).
    fn link_planned(&mut self, record: &LinkRecord) -> Result<()>;
    /// A link was reverted and no longer needs tracking.
    fn link_reverted(&mut self, target: &Path) -> Result<()>;
    /// A directory is about to be created to host links.
    fn dir_planned(&mut self, dir: &Path) -> Result<()>;
    /// All created directories were cleaned up.
    fn dirs_cleared(&mut self) -> Result<()>;
}

/// A journal that persists nothing (tests).
#[cfg(test)]
#[derive(Debug, Default, Clone, Copy)]
pub struct NullJournal;

#[cfg(test)]
impl Journal for NullJournal {
    fn link_planned(&mut self, _: &LinkRecord) -> Result<()> {
        Ok(())
    }
    fn link_reverted(&mut self, _: &Path) -> Result<()> {
        Ok(())
    }
    fn dir_planned(&mut self, _: &Path) -> Result<()> {
        Ok(())
    }
    fn dirs_cleared(&mut self) -> Result<()> {
        Ok(())
    }
}
