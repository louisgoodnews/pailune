//! Profiles: named installation sets (which mods are enabled, in which order).

pub mod ops;

use serde::{Deserialize, Serialize};

/// One mod's slot inside a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSlot {
    /// Staged mod name.
    pub name: String,
    /// Whether the mod is enabled in this profile.
    pub enabled: bool,
}

/// A named snapshot of the mod list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Unique display name.
    pub name: String,
    /// Mods in load order.
    pub slots: Vec<ProfileSlot>,
    /// Steam-style launch options, e.g. `WINEDLLOVERRIDES="winmm=n,b" %command% -dx12`.
    pub launch_options: String,
}

/// All profiles plus the one last loaded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileBook {
    /// Profiles sorted by name.
    pub profiles: Vec<Profile>,
    /// Name of the most recently loaded or saved profile.
    pub active: Option<String>,
}

/// Result of applying a profile to the current registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOutcome {
    /// Profile entries whose mod is no longer staged.
    pub missing: Vec<String>,
}
