//! Reading and writing `config.toml`.

use super::Config;
use crate::error::{IoContext, Result};
use std::fs;
use std::path::Path;

/// Loads the configuration, creating the file with defaults if missing.
pub fn load_or_create(path: &Path) -> Result<Config> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(toml::from_str(&text)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let config = Config::default();
            save(path, &config)?;
            Ok(config)
        }
        Err(err) => Err(err).at(path),
    }
}

/// Writes the configuration atomically via a temporary sibling file.
pub fn save(path: &Path, config: &Config) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).at(dir)?;
    }
    let text = format!(
        "# Pylune configuration\n\n{}",
        toml::to_string_pretty(config)?
    );
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, text).at(&tmp)?;
    fs::rename(&tmp, path).at(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_fills_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let created = load_or_create(&path).unwrap();
        assert_eq!(created.nexus.domain, "crimsondesert");
        fs::write(&path, "[game]\napp_id = 7\n").unwrap();
        let partial = load_or_create(&path).unwrap();
        assert_eq!(partial.game.app_id, 7);
        assert_eq!(partial.nexus.domain, Config::default().nexus.domain);
    }
}
