//! Locating Proton and the Steam client from `compatdata/<id>/config_info`.

use super::ProtonSetup;
use crate::steam::SteamInfo;
use std::fs;
use std::path::PathBuf;

/// Extracts the Proton directory from a `.../<Proton>/files/...` line.
fn proton_dir(lines: &[&str]) -> Option<PathBuf> {
    lines
        .iter()
        .find_map(|line| {
            line.split_once("/files/")
                .map(|(dir, _)| PathBuf::from(dir))
        })
        .filter(|dir| dir.join("proton").is_file())
}

/// The Steam client directory: the first existing directory line that is
/// not inside Proton, falling back to the usual locations.
fn steam_dir(lines: &[&str]) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    lines
        .iter()
        .filter(|line| line.starts_with('/') && !line.contains("/files/"))
        .map(PathBuf::from)
        .chain([home.join(".local/share/Steam"), home.join(".steam/steam")])
        .find(|dir| dir.join("steamapps").is_dir())
}

/// Reads the Proton setup for the game, if its prefix was ever created.
pub fn setup(info: &SteamInfo) -> Option<ProtonSetup> {
    let compat_data = info.prefix_dir.as_ref()?.parent()?.to_path_buf();
    let text = fs::read_to_string(compat_data.join("config_info")).ok()?;
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    Some(ProtonSetup {
        proton_dir: proton_dir(&lines)?,
        steam_dir: steam_dir(&lines)?,
        compat_data,
    })
}
