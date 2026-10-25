//! Locating Proton, its Steam Linux Runtime and the Steam client, so a
//! direct launch runs the same tool chain Steam itself would.

use super::ProtonSetup;
use crate::steam::vdf::{self, Vdf};
use crate::steam::{SteamInfo, library};
use std::fs;
use std::path::{Path, PathBuf};

/// The verb Steam uses to start a game and wait for it.
const VERB: &str = "waitforexitandrun";

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

/// A tool's `toolmanifest.vdf` manifest node.
fn manifest(tool_dir: &Path) -> Option<Vdf> {
    let text = fs::read_to_string(tool_dir.join("toolmanifest.vdf")).ok()?;
    vdf::parse(&text).get("manifest").cloned()
}

/// Expands a manifest `commandline` (e.g. `/proton %verb%`) for `tool_dir`.
fn expand(tool_dir: &Path, commandline: &str) -> Vec<String> {
    commandline
        .split_whitespace()
        .enumerate()
        .map(|(i, part)| match i {
            0 => format!("{}{part}", tool_dir.display()),
            _ => part.replace("%verb%", VERB),
        })
        .collect()
}

/// Builds the full tool chain, following `require_tool_appid` (at most a
/// few levels: Proton → Steam Linux Runtime).
pub fn tool_chain(
    tool_dir: &Path,
    find_tool: &dyn Fn(u32) -> Option<PathBuf>,
    depth: usize,
) -> Vec<String> {
    let Some(manifest) = manifest(tool_dir) else {
        return vec![tool_dir.join("proton").display().to_string(), VERB.into()];
    };
    let own = expand(
        tool_dir,
        manifest
            .get("commandline")
            .and_then(Vdf::as_str)
            .unwrap_or("/proton %verb%"),
    );
    let outer = manifest
        .get("require_tool_appid")
        .and_then(Vdf::as_str)
        .and_then(|id| id.parse().ok())
        .filter(|_| depth > 0)
        .and_then(|id| find_tool(id))
        .map(|dir| tool_chain(&dir, find_tool, depth - 1))
        .unwrap_or_default();
    outer.into_iter().chain(own).collect()
}

/// Reads the Proton setup for the game, if its prefix was ever created.
pub fn setup(info: &SteamInfo) -> Option<ProtonSetup> {
    let compat_data = info.prefix_dir.as_ref()?.parent()?.to_path_buf();
    let text = fs::read_to_string(compat_data.join("config_info")).ok()?;
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let proton_dir = proton_dir(&lines)?;
    let command = tool_chain(&proton_dir, &library::find_app, 3);
    Some(ProtonSetup {
        steam_dir: steam_dir(&lines)?,
        proton_dir,
        compat_data,
        command,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chains_proton_into_the_runtime() {
        let tmp = tempfile::tempdir().unwrap();
        let (proton, slr) = (tmp.path().join("Proton"), tmp.path().join("SLR"));
        fs::create_dir_all(&proton).unwrap();
        fs::create_dir_all(&slr).unwrap();
        fs::write(
            proton.join("toolmanifest.vdf"),
            r#""manifest" { "commandline" "/proton %verb%" "require_tool_appid" "4183110" }"#,
        )
        .unwrap();
        fs::write(
            slr.join("toolmanifest.vdf"),
            r#""manifest" { "commandline" "/_v2-entry-point --verb=%verb% --" }"#,
        )
        .unwrap();
        let find = |id: u32| (id == 4_183_110).then(|| slr.clone());
        let chain = tool_chain(&proton, &find, 3);
        let expected = [
            format!("{}/_v2-entry-point", slr.display()),
            "--verb=waitforexitandrun".into(),
            "--".into(),
            format!("{}/proton", proton.display()),
            "waitforexitandrun".into(),
        ];
        assert_eq!(chain, expected);
    }
}

#[cfg(test)]
mod machine {
    /// Prints the chain for the local install (`cargo test -- --ignored`); launches nothing.
    #[test]
    #[ignore = "depends on the local Steam installation"]
    fn resolves_the_local_tool_chain() {
        let mut config = crate::config::Config::default();
        config.game.dir =
            crate::steam::library::find_game(config.game.app_id).expect("game installed");
        let setup = super::setup(&crate::steam::locate::resolve(&config)).expect("proton setup");
        println!("{:#?}", setup.command);
        assert!(
            setup
                .command
                .first()
                .is_some_and(|c| c.ends_with("_v2-entry-point"))
        );
    }
}
