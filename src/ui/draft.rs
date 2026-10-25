//! Converting between the committed [`Config`] and the editable settings draft.

use super::{SettingsDraft, SettingsPage};
use crate::config::Config;
use crate::error::{PyluneError, Result};
use std::path::PathBuf;

impl SettingsDraft {
    /// Mirrors a committed [`Config`] into an editable draft.
    pub fn from_config(config: &Config) -> Self {
        Self {
            game_dir: config.game.dir.display().to_string(),
            config: config.clone(),
            page: SettingsPage::default(),
        }
    }

    /// Validates the draft into a [`Config`].
    pub fn to_config(&self) -> Result<Config> {
        let invalid = |what: &str| Err(PyluneError::Invalid(what.to_string()));
        let mut next = self.config.clone();
        next.game.dir = PathBuf::from(self.game_dir.trim());
        next.nexus.domain = next.nexus.domain.trim().to_lowercase();
        next.launch.default_options = next.launch.default_options.trim().to_string();
        next.logging.level = next.logging.level.trim().to_string();
        let domain_ok = next
            .nexus
            .domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
        match () {
            _ if self.game_dir.trim().is_empty() => invalid("the game directory is required"),
            _ if next.game.app_id == 0 => invalid("the Steam app id must not be 0"),
            _ if next.nexus.domain.is_empty() || !domain_ok => {
                invalid("the Nexus game domain may only contain letters, digits and '-'")
            }
            _ if !(1..=16).contains(&next.jobs.max_parallel) => {
                invalid("parallel jobs must be between 1 and 16")
            }
            _ if tracing_subscriber::EnvFilter::try_new(&next.logging.level).is_err() => {
                invalid("the log level is not a valid filter (try info or pylune=debug)")
            }
            _ => Ok(next),
        }
    }

    /// The draft differs from the committed configuration.
    pub fn is_changed(&self, config: &Config) -> bool {
        self.config != *config || std::path::Path::new(self.game_dir.trim()) != config.game.dir
    }

    /// Resets one category to its defaults (the game folder is kept).
    pub fn reset(&mut self, page: SettingsPage) {
        let defaults = Config::default();
        let config = &mut self.config;
        match page {
            SettingsPage::Appearance => config.ui = defaults.ui,
            SettingsPage::Behavior => config.behavior = defaults.behavior,
            SettingsPage::Mods => config.mods = defaults.mods,
            SettingsPage::Game => {
                config.game.app_id = defaults.game.app_id;
                config.launch = defaults.launch;
            }
            SettingsPage::Nexus => config.nexus.domain = defaults.nexus.domain,
            SettingsPage::Backups => config.backup = defaults.backup,
            SettingsPage::Advanced => {
                config.jobs = defaults.jobs;
                config.logging = defaults.logging;
            }
            SettingsPage::Diagnostics => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_resets() {
        let mut draft = SettingsDraft::from_config(&Config::default());
        assert!(draft.to_config().is_err(), "no game folder yet");
        draft.game_dir = "/games/cd".into();
        draft.config.nexus.domain = " CrimsonDesert ".into();
        assert_eq!(draft.to_config().unwrap().nexus.domain, "crimsondesert");
        draft.config.logging.level = "pylune=nonsense=x".into();
        assert!(draft.to_config().is_err());
        draft.reset(SettingsPage::Advanced);
        draft.config.mods.trash_keep = 3;
        draft.reset(SettingsPage::Mods);
        assert_eq!(draft.config.mods, Default::default());
        assert!(draft.to_config().is_ok());
    }
}
