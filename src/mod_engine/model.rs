//! Plain data describing staged mods. Behaviour lives in sibling modules.

use std::path::PathBuf;

/// Broad category of a mod, derived from the files it contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModKind {
    /// JSON patches for archived game data (needs the PAZ pipeline).
    Json,
    /// RAW trees or loose game files (needs the PAZ pipeline).
    Raw,
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
    /// Number of files that can be linked into the game.
    pub file_count: usize,
    /// Number of files that need packing into PAZ archives (not deployable yet).
    pub paz_files: usize,
    /// Unix timestamp of the (last) installation.
    pub installed_at: i64,
    /// Original file, folder or Nexus link the mod was installed from.
    pub source: String,
    /// Nexus origin, if downloaded from Nexus Mods.
    pub nexus: Option<NexusRef>,
    /// Chosen option folder of a multi-variant mod (`None`: the first).
    pub variant: Option<String>,
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
    pub paz_files: usize,
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
    /// To this 1-based load-order number.
    Position(usize),
}

/// A staged file and its size, as shown in the mod details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedFile {
    pub rel: PathBuf,
    pub bytes: u64,
    /// How the file is used.
    pub placement: crate::mod_engine::plan::Placement,
}

/// Everything the details panel shows about one mod.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModDetails {
    pub files: Vec<StagedFile>,
    pub total_bytes: u64,
    /// Readme / changelog candidates, relative to the mod root.
    pub readmes: Vec<PathBuf>,
    /// Option folders of a multi-variant mod (empty otherwise).
    pub variants: Vec<String>,
}
