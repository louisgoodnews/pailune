//! First-run game discovery: use the configured directory if valid,
//! otherwise ask Steam's library configuration.

use super::{Config, file};
use crate::steam::{GameStatus, library, locate};
use std::path::{Path, PathBuf};

/// How the game directory was settled at start-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discovery {
    /// The configured directory contains the game.
    Configured,
    /// Found through Steam and saved to `config.toml`.
    Detected(PathBuf),
    /// Neither worked: the user has to pick the folder.
    NotFound,
}

/// Validates or detects the game directory, saving a detected one.
pub fn resolve_game(config: Config, config_file: &Path) -> (Config, Discovery) {
    let ready = |c: &Config| locate::game_status(&locate::resolve(c)) == GameStatus::Ready;
    if !config.game.dir.as_os_str().is_empty() && ready(&config) {
        return (config, Discovery::Configured);
    }
    match library::find_game(config.game.app_id) {
        Some(dir) => {
            let mut detected = config;
            detected.game.dir = dir.clone();
            if let Err(err) = file::save(config_file, &detected) {
                tracing::warn!(%err, "could not save the detected game directory");
            }
            (detected, Discovery::Detected(dir))
        }
        None => (config, Discovery::NotFound),
    }
}
