//! Pure-ish helpers that derive [`SteamInfo`] and probe the installation.

use super::{AsiLoader, GameStatus, SteamInfo};
use crate::config::Config;
use std::path::Path;

/// Name of the game executable inside `bin64/`.
const GAME_EXE: &str = "CrimsonDesert.exe";

/// Loader DLL names in priority order, paired with their variant.
const LOADERS: [(&str, AsiLoader); 3] = [
    ("winmm.dll", AsiLoader::Winmm),
    ("version.dll", AsiLoader::Version),
    ("dinput8.dll", AsiLoader::Dinput8),
];

/// Derives every relevant path from the user configuration.
///
/// The Proton prefix is expected at `<steamapps>/compatdata/<app_id>/pfx`,
/// where `<steamapps>` is two levels above the game directory
/// (`steamapps/common/<game>`).
pub fn resolve(config: &Config) -> SteamInfo {
    let game_dir = config.game_dir.clone();
    let prefix_dir = game_dir
        .parent()
        .and_then(Path::parent)
        .map(|steamapps| {
            steamapps
                .join("compatdata")
                .join(config.app_id.to_string())
                .join("pfx")
        })
        .filter(|pfx| pfx.is_dir());
    SteamInfo {
        app_id: config.app_id,
        bin64_dir: game_dir.join("bin64"),
        meta_dir: game_dir.join("meta"),
        prefix_dir,
        game_dir,
    }
}

/// Classifies the installation by checking for the executable and metadata.
pub fn game_status(info: &SteamInfo) -> GameStatus {
    let exe = info.bin64_dir.join(GAME_EXE).is_file();
    let meta = info.meta_dir.join("0.papgt").is_file();
    match (info.game_dir.is_dir(), exe && meta) {
        (false, _) => GameStatus::Missing,
        (true, false) => GameStatus::Unrecognised,
        (true, true) => GameStatus::Ready,
    }
}

/// Detects the first ASI loader proxy DLL present in `bin64/`.
pub fn detect_loader(info: &SteamInfo) -> AsiLoader {
    LOADERS
        .iter()
        .find(|(dll, _)| info.bin64_dir.join(dll).exists())
        .map_or(AsiLoader::Absent, |(_, loader)| *loader)
}

/// Steam launch options required for Wine to prefer the native loader DLL.
pub fn launch_options(loader: AsiLoader) -> Option<String> {
    let dll = match loader {
        AsiLoader::Winmm => "winmm",
        AsiLoader::Version => "version",
        AsiLoader::Dinput8 => "dinput8",
        AsiLoader::Absent => return None,
    };
    Some(format!("WINEDLLOVERRIDES=\"{dll}=n,b\" %command%"))
}
