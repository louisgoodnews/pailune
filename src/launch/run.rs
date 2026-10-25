//! Spawning the Steam launch request and probing `/proc` for the game.

use super::{GameProcess, LaunchMethod};
use crate::error::{PyluneError, Result};
use std::fs;
use std::process::{Command, Stdio};

/// Executable name the game runs as under Proton.
const GAME_PROCESS: &str = "CrimsonDesert.exe";

/// The Steam URL that launches `app_id`.
pub fn launch_url(app_id: u32) -> String {
    format!("steam://rungameid/{app_id}")
}

/// Spawns `program url` fully detached from Pylune's stdio.
fn spawn(program: &str, url: &str) -> bool {
    Command::new(program)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

/// Asks Steam to start the game, trying the Steam CLI first.
pub fn launch(app_id: u32) -> Result<LaunchMethod> {
    let url = launch_url(app_id);
    [
        ("steam", LaunchMethod::SteamCli),
        ("xdg-open", LaunchMethod::XdgOpen),
    ]
    .into_iter()
    .find(|(program, _)| spawn(program, &url))
    .map(|(_, method)| method)
    .ok_or_else(|| PyluneError::Invalid("neither `steam` nor `xdg-open` could be started".into()))
}

/// Whether a process command line mentions the game executable.
fn cmdline_matches(raw: &[u8]) -> bool {
    raw.split(|b| *b == 0)
        .any(|arg| arg.ends_with(GAME_PROCESS.as_bytes()))
}

/// Scans `/proc` for a running `CrimsonDesert.exe`.
pub fn game_process() -> GameProcess {
    let running = fs::read_dir("/proc")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .bytes()
                        .all(|b| b.is_ascii_digit())
                })
                .filter_map(|e| fs::read(e.path().join("cmdline")).ok())
                .any(|raw| cmdline_matches(&raw))
        })
        .unwrap_or(false);
    match running {
        true => GameProcess::Running,
        false => GameProcess::Stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_proton_cmdlines() {
        assert!(cmdline_matches(
            b"Z:\\games\\Crimson Desert\\bin64\\CrimsonDesert.exe\0-dx12\0"
        ));
        assert!(!cmdline_matches(
            b"/usr/bin/steam\0steam://rungameid/3321460\0"
        ));
        assert_eq!(launch_url(3_321_460), "steam://rungameid/3321460");
    }
}
