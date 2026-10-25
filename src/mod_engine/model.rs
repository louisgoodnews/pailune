//! Plain data describing staged mods. Behaviour lives in sibling modules.

use std::path::PathBuf;

/// Broad category of a mod, derived from the files it contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Where a staged mod came from on Nexus Mods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NexusRef {
    pub mod_id: u64,
    pub file_id: u64,
    /// Mod version at download time.
    pub version: String,
}

/// One mod in the staging folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModEntry {
    /// Directory name inside the staging folder; also the display name.
    pub name: String,
    /// Detected category.
    pub kind: ModKind,
    /// Whether the mod should be linked into the game on the next apply.
    pub enabled: bool,
    /// Number of deployable files.
    pub file_count: usize,
    /// Unix timestamp of the (last) installation.
    pub installed_at: i64,
    /// Original file, folder or Nexus link the mod was installed from.
    pub source: String,
    /// Nexus origin, if downloaded from Nexus Mods.
    pub nexus: Option<NexusRef>,
}

/// Ordered list of mods. Later entries win file conflicts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    /// Mods in load order.
    pub mods: Vec<ModEntry>,
}

/// What a staging scan found for one mod folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scanned {
    pub kind: ModKind,
    pub file_count: usize,
}

/// Staging scan result: mod folder name → classification.
pub type Scan = std::collections::BTreeMap<String, Scanned>;

/// Where a mod is moved in the load order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveTo {
    Top,
    Up,
    Down,
    Bottom,
}

/// A staged file and its size, as shown in the mod details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedFile {
    pub rel: PathBuf,
    pub bytes: u64,
    /// `false` for documentation and other files that are never linked.
    pub deployable: bool,
}

/// Everything the details panel shows about one mod.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModDetails {
    pub files: Vec<StagedFile>,
    pub total_bytes: u64,
    /// Readme / changelog candidates, relative to the mod root.
    pub readmes: Vec<PathBuf>,
}
