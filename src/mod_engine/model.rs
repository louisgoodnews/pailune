//! Plain data describing staged mods. Behaviour lives in sibling modules.

use serde::{Deserialize, Serialize};

/// Broad category of a mod, derived from the files it contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModKind {
    /// Only JSON patch files.
    Json,
    /// ASI plugins and/or DLLs destined for `bin64/`.
    Asi,
    /// Files mirroring the game layout (archive folders, `meta/`, ...).
    Overlay,
    /// A combination of the above.
    Mixed,
    /// Nothing deployable was found.
    Empty,
}

/// One mod in the staging folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModEntry {
    /// Directory name inside the staging folder; also the display name.
    pub name: String,
    /// Detected category.
    pub kind: ModKind,
    /// Whether the mod should be linked into the game on the next apply.
    pub enabled: bool,
    /// Number of deployable files.
    pub file_count: usize,
}

/// Ordered list of mods. Later entries win file conflicts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Registry {
    /// Mods in load order.
    pub mods: Vec<ModEntry>,
}
