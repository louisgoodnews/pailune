//! Pylune's own archive overlay (`docs/paz-format.md` §9).
//!
//! RAW and loose game files of enabled mods are packed into one Pylune
//! archive group, and prebuilt groups shipped by mods are registered under
//! free numbers. A rebuilt `meta/0.papgt` (from the vanilla backup) lists
//! these groups first, so the game prefers their files. Everything is built
//! in Pylune's data folder and deployed as symlinks like any mod file, so
//! Unlink and Clean Slate revert it exactly.

pub mod build;
pub mod collect;
pub mod patches;
pub mod textures;

#[cfg(test)]
mod machine;

use crate::mod_engine::plan::LinkOp;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Archive content gathered from the enabled mods, in load order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverlayInput {
    /// Virtual path → staged file (later mods win).
    pub packs: BTreeMap<String, PathBuf>,
    /// Prebuilt group folders, in load order.
    pub prebuilt: Vec<PathBuf>,
    /// Byte-patch JSON files `(mod name, file)`, in load order.
    pub patches: Vec<(String, PathBuf)>,
    /// CSS patches `(mod name, target stylesheet, file)`, in load order.
    pub css_patches: Vec<(String, String, PathBuf)>,
    /// Virtual paths provided by more than one mod.
    pub conflicts: usize,
}

/// What the overlay contributes to a deployment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverlayPlan {
    /// Symlinks for the group folders and the generated `0.papgt`.
    pub links: Vec<LinkOp>,
    /// Files packed into Pylune's group.
    pub packed: usize,
    /// Of those, files that replace a game file.
    pub replaced: usize,
    /// Registered group names, highest priority first.
    pub groups: Vec<String>,
    /// Files left out, with the reason.
    pub skipped: Vec<String>,
    /// Results of JSON patches, per mod.
    pub patch_notes: Vec<patches::PatchNote>,
}

impl OverlayInput {
    /// Whether the enabled mods need no overlay at all.
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
            && self.prebuilt.is_empty()
            && self.patches.is_empty()
            && self.css_patches.is_empty()
    }
}
