//! Spawning the game and probing `/proc` for it.

use super::{GameProcess, LaunchMethod, LaunchSpec, ProtonSetup};
use crate::error::{PyluneError, Result};
use crate::steam::SteamInfo;
use std::fs;
use std::process::{Command, Stdio};

/// Executable name the game runs as under Proton.
const GAME_PROCESS: &str = "CrimsonDesert.exe";

/// The Steam URL that launches `app_id`.
pub fn launch_url(app_id: u32) -> String {
    format!("steam://rungameid/{app_id}")
}

/// Spawns a command fully detached from Pylune's stdio.
fn spawn(mut command: Command) -> std::io::Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
}

/// Through the Steam client: `steam -applaunch`, else the URL handler.
fn via_steam(info: &SteamInfo, spec: &LaunchSpec) -> Result<LaunchMethod> {
    let mut steam = Command::new("steam");
    steam
        .arg("-applaunch")
        .arg(info.app_id.to_string())
        .args(&spec.args);
    let mut xdg = Command::new("xdg-open");
    xdg.arg(launch_url(info.app_id));
    spawn(steam)
        .map(|()| LaunchMethod::SteamCli)
        .or_else(|_| spawn(xdg).map(|()| LaunchMethod::XdgOpen))
        .map_err(|e| PyluneError::Invalid(format!("could not start Steam: {e}")))
}

/// Directly through Proton with the extra environment from `spec`.
fn via_proton(info: &SteamInfo, setup: &ProtonSetup, spec: &LaunchSpec) -> Result<LaunchMethod> {
    let app_id = info.app_id.to_string();
    let mut proton = Command::new(setup.proton_dir.join("proton"));
    proton
        .arg("waitforexitandrun")
        .arg(info.bin64_dir.join(GAME_PROCESS))
        .args(&spec.args)
        .current_dir(&info.bin64_dir)
        .env("STEAM_COMPAT_DATA_PATH", &setup.compat_data)
        .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &setup.steam_dir)
        .env("SteamAppId", &app_id)
        .env("SteamGameId", &app_id)
        .envs(spec.env.iter().map(|(k, v)| (k, v)));
    spawn(proton)
        .map(|()| LaunchMethod::ProtonDirect)
        .map_err(|e| PyluneError::Invalid(format!("could not start Proton: {e}")))
}

/// Starts the game, choosing the method the launch options require.
pub fn launch(
    info: &SteamInfo,
    setup: Option<&ProtonSetup>,
    spec: &LaunchSpec,
) -> Result<LaunchMethod> {
    match (spec.env.is_empty(), setup) {
        (true, _) => via_steam(info, spec),
        (false, Some(setup)) => via_proton(info, setup, spec),
        (false, None) => Err(PyluneError::Invalid(
            "launch options set environment variables, but no Proton installation was found \
             for this game — start it once from Steam first"
                .into(),
        )),
    }
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
