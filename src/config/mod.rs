//! User configuration, stored as TOML in `~/.config/pylune/config.toml`.

pub mod file;
pub mod paths;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Steam application id of *Crimson Desert*.
pub const CRIMSON_DESERT_APP_ID: u32 = 3_321_460;

/// Nexus Mods game domain of *Crimson Desert*.
pub const NEXUS_DOMAIN: &str = "crimsondesert";

/// Default install location on this machine.
const DEFAULT_GAME_DIR: &str =
    "/mnt/nvme0n1/nvme0n1p1/louisgoodnews/steamapps/common/Crimson Desert";

/// `[game]` — where and what the game is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameConfig {
    /// Root of the installation (contains `bin64/`, `meta/`).
    pub dir: PathBuf,
    /// Steam app id, used for launching and locating the Proton prefix.
    pub app_id: u32,
}

/// `[nexus]` — Nexus Mods connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NexusConfig {
    /// Game domain (`nexusmods.com/<domain>`).
    pub domain: String,
}

/// `[launch]` — how the game is started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchConfig {
    /// Deploy pending mod changes before launching.
    pub apply_before_launch: bool,
    /// Steam-style launch options used when the active profile has none.
    pub default_options: String,
}

/// `[jobs]` — background work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct JobsConfig {
    /// Number of worker threads running jobs in parallel.
    pub max_parallel: usize,
}

/// `[logging]` — diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// `tracing` filter for the log file, e.g. `info` or `pylune=debug`.
    pub level: String,
}

/// Complete configuration file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub game: GameConfig,
    pub nexus: NexusConfig,
    pub launch: LaunchConfig,
    pub jobs: JobsConfig,
    pub logging: LoggingConfig,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(DEFAULT_GAME_DIR),
            app_id: CRIMSON_DESERT_APP_ID,
        }
    }
}

impl Default for NexusConfig {
    fn default() -> Self {
        Self {
            domain: NEXUS_DOMAIN.into(),
        }
    }
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            apply_before_launch: true,
            default_options: String::new(),
        }
    }
}

impl Default for JobsConfig {
    fn default() -> Self {
        let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
        Self {
            max_parallel: cores.clamp(2, 4),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
        }
    }
}
