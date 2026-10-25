//! Spawning the game and probing `/proc` for it.

use super::{GameProcess, LaunchMethod, LaunchSpec, ProtonSetup};
use crate::error::{PyluneError, Result};
use crate::steam::SteamInfo;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Executable name the game runs as under Proton.
const GAME_PROCESS: &str = "CrimsonDesert.exe";

/// The Steam URL that launches `app_id`.
pub fn launch_url(app_id: u32) -> String {
    format!("steam://rungameid/{app_id}")
}

/// Spawns a command fully detached from Pylune's stdio.
///
/// A thread reaps the child when it exits, so no zombie processes pile up.
pub fn spawn(mut command: Command) -> std::io::Result<()> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || drop(child.wait()));
    Ok(())
}

/// `steam://run/<id>//<args>/` passes arguments even through the URL handler.
fn run_url(app_id: u32, args: &[String]) -> String {
    match args {
        [] => launch_url(app_id),
        args => {
            let joined = args
                .join(" ")
                .replace('%', "%25")
                .replace(' ', "%20")
                .replace('/', "%2F");
            format!("steam://run/{app_id}//{joined}/")
        }
    }
}

/// Through the Steam client: `steam -applaunch`, else the URL handler.
fn via_steam(info: &SteamInfo, spec: &LaunchSpec) -> Result<LaunchMethod> {
    let mut steam = Command::new("steam");
    steam
        .arg("-applaunch")
        .arg(info.app_id.to_string())
        .args(&spec.args);
    let mut xdg = Command::new("xdg-open");
    xdg.arg(run_url(info.app_id, &spec.args));
    spawn(steam)
        .map(|()| LaunchMethod::SteamCli)
        .or_else(|_| spawn(xdg).map(|()| LaunchMethod::XdgOpen))
        .map_err(|e| PyluneError::Invalid(format!("could not start Steam: {e}")))
}

/// Asks Steam to verify (repair) the game files (`steam://validate/<id>`).
pub fn verify_files(app_id: u32) -> Result<()> {
    let url = format!("steam://validate/{app_id}");
    let mut steam = Command::new("steam");
    steam.arg(&url);
    let mut xdg = Command::new("xdg-open");
    xdg.arg(&url);
    spawn(steam)
        .or_else(|_| spawn(xdg))
        .map_err(|e| PyluneError::Invalid(format!("could not start Steam: {e}")))
}

/// Directly through Proton (inside its Steam Linux Runtime, as Steam does),
/// with the extra environment and wrapper commands from `spec`.
fn via_proton(info: &SteamInfo, setup: &ProtonSetup, spec: &LaunchSpec) -> Result<LaunchMethod> {
    let app_id = info.app_id.to_string();
    let library = info
        .game_dir
        .ancestors()
        .nth(3)
        .map(PathBuf::from)
        .unwrap_or_default();
    let argv: Vec<String> = spec
        .wrapper
        .iter()
        .cloned()
        .chain(setup.command.iter().cloned())
        .chain(std::iter::once(
            info.bin64_dir.join(GAME_PROCESS).display().to_string(),
        ))
        .chain(spec.args.iter().cloned())
        .collect();
    let Some((program, rest)) = argv.split_first() else {
        return Err(PyluneError::Invalid("empty launch command".into()));
    };
    let mut command = Command::new(program);
    command
        .args(rest)
        .current_dir(&info.bin64_dir)
        .env("STEAM_COMPAT_DATA_PATH", &setup.compat_data)
        .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &setup.steam_dir)
        .env("STEAM_COMPAT_INSTALL_PATH", &info.game_dir)
        .env("STEAM_COMPAT_LIBRARY_PATHS", &library)
        .env("STEAM_COMPAT_APP_ID", &app_id)
        .env("SteamAppId", &app_id)
        .env("SteamGameId", &app_id)
        .envs(spec.env.iter().map(|(k, v)| (k, v)));
    tracing::info!(?argv, "launching through Proton directly");
    spawn(command)
        .map(|()| LaunchMethod::ProtonDirect)
        .map_err(|e| PyluneError::Invalid(format!("could not start {program}: {e}")))
}

/// Starts the game, choosing the method the launch options require:
/// environment variables or wrappers need a direct Proton launch, since
/// `steam -applaunch` can only pass game arguments.
pub fn launch(
    info: &SteamInfo,
    setup: Option<&ProtonSetup>,
    spec: &LaunchSpec,
) -> Result<LaunchMethod> {
    let direct = !spec.env.is_empty() || !spec.wrapper.is_empty();
    match (direct, setup) {
        (false, _) => via_steam(info, spec),
        (true, Some(setup)) => via_proton(info, setup, spec),
        (true, None) => Err(PyluneError::Invalid(
            "launch options need a direct Proton launch, but no Proton installation was found \
             for this game — start it once from Steam first"
                .into(),
        )),
    }
}

/// Whether a process command line mentions the game executable.
///
/// Only the program itself (argv\[0\]) counts, compared case-insensitively
/// after the last `/` or `\` — an editor that merely has the file open
/// does not block deployment.
fn cmdline_matches(raw: &[u8]) -> bool {
    let program = raw.split(|b| *b == 0).next().unwrap_or_default();
    let name = program
        .rsplit(|b| *b == b'/' || *b == b'\\')
        .next()
        .unwrap_or_default();
    name.eq_ignore_ascii_case(GAME_PROCESS.as_bytes())
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
        assert!(cmdline_matches(b"C:\\Games\\crimsondesert.EXE\0"));
        assert!(!cmdline_matches(
            b"/usr/bin/kate\0/games/bin64/CrimsonDesert.exe\0"
        ));
        assert_eq!(
            run_url(1, &["-dx12".into(), "a b".into()]),
            "steam://run/1//-dx12%20a%20b/"
        );
    }
}
