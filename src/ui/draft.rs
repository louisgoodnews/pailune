//! Converting between the committed [`Config`] and the editable settings draft.

use super::SettingsDraft;
use crate::config::Config;
use crate::error::{PyluneError, Result};
use std::path::PathBuf;

impl SettingsDraft {
    /// Mirrors a committed [`Config`] into editable strings.
    pub fn from_config(config: &Config) -> Self {
        Self {
            game_dir: config.game_dir.display().to_string(),
            app_id: config.app_id.to_string(),
            json_subdir: config.json_subdir.display().to_string(),
            nexus_domain: config.nexus_domain.clone(),
            apply_before_launch: config.apply_before_launch,
        }
    }

    /// Validates the draft into a [`Config`].
    pub fn to_config(&self) -> Result<Config> {
        let app_id = self.app_id.trim().parse().map_err(|_| {
            PyluneError::Invalid(format!("'{}' is not a valid Steam app id", self.app_id))
        })?;
        let json_subdir = PathBuf::from(self.json_subdir.trim());
        match json_subdir.is_absolute() || self.game_dir.trim().is_empty() {
            true => Err(PyluneError::Invalid(
                "game directory is required and the JSON folder must be relative".into(),
            )),
            false => Ok(Config {
                game_dir: PathBuf::from(self.game_dir.trim()),
                app_id,
                json_subdir,
                nexus_domain: self.nexus_domain.trim().to_lowercase(),
                apply_before_launch: self.apply_before_launch,
            }),
        }
    }
}
