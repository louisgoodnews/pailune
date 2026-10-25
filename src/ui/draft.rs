//! Converting between the committed [`Config`] and the editable settings draft.

use super::SettingsDraft;
use crate::config::{Config, GameConfig, JobsConfig, LaunchConfig, LoggingConfig, NexusConfig};
use crate::error::{PyluneError, Result};
use std::path::PathBuf;

impl SettingsDraft {
    /// Mirrors a committed [`Config`] into editable strings.
    pub fn from_config(config: &Config) -> Self {
        Self {
            game_dir: config.game.dir.display().to_string(),
            app_id: config.game.app_id.to_string(),
            nexus_domain: config.nexus.domain.clone(),
            apply_before_launch: config.launch.apply_before_launch,
            default_options: config.launch.default_options.clone(),
            max_parallel: config.jobs.max_parallel.to_string(),
            log_level: config.logging.level.clone(),
        }
    }

    /// Validates the draft into a [`Config`].
    pub fn to_config(&self) -> Result<Config> {
        let invalid = |what: &str| PyluneError::Invalid(what.to_string());
        let app_id = self
            .app_id
            .trim()
            .parse()
            .map_err(|_| invalid("Steam app id must be a number"))?;
        let max_parallel: usize = self
            .max_parallel
            .trim()
            .parse()
            .ok()
            .filter(|n| (1..=16).contains(n))
            .ok_or_else(|| invalid("parallel jobs must be between 1 and 16"))?;
        if self.game_dir.trim().is_empty() {
            return Err(invalid("the game directory is required"));
        }
        Ok(Config {
            game: GameConfig {
                dir: PathBuf::from(self.game_dir.trim()),
                app_id,
            },
            nexus: NexusConfig {
                domain: self.nexus_domain.trim().to_lowercase(),
            },
            launch: LaunchConfig {
                apply_before_launch: self.apply_before_launch,
                default_options: self.default_options.trim().to_string(),
            },
            jobs: JobsConfig { max_parallel },
            logging: LoggingConfig {
                level: self.log_level.trim().to_string(),
            },
        })
    }
}
