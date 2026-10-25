//! User configuration: where the game lives and how mods are laid out.

pub mod store;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Steam application id of *Crimson Desert*.
pub const CRIMSON_DESERT_APP_ID: u32 = 3_321_460;

/// Default install location on this machine.
const DEFAULT_GAME_DIR: &str =
    "/mnt/nvme0n1/nvme0n1p1/louisgoodnews/steamapps/common/Crimson Desert";

/// Persistent settings, stored as `config.json` in the Pylune data directory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Root of the *Crimson Desert* installation (contains `bin64/`, `meta/`).
    pub game_dir: PathBuf,
    /// Steam app id, used to locate the Proton prefix under `compatdata/`.
    pub app_id: u32,
    /// Game-relative directory that loose JSON mods are linked into.
    pub json_subdir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_dir: PathBuf::from(DEFAULT_GAME_DIR),
            app_id: CRIMSON_DESERT_APP_ID,
            json_subdir: PathBuf::from("bin64/mods"),
        }
    }
}
