//! Steam's own launch options for a game (`userdata/<id>/config/
//! localconfig.vdf`). Under Proton an ASI loader such as `winmm.dll` is only
//! used when they contain `WINEDLLOVERRIDES="winmm=n,b"`; without it no ASI
//! plugin runs, which is easy to miss.

use super::vdf::{self, Vdf};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// The `LaunchOptions` of `app_id` in a parsed `localconfig.vdf`: `""` when
/// the game has an entry without options, `None` without an entry.
fn find(node: &Vdf, app_id: &str) -> Option<String> {
    node.entries().iter().find_map(|(key, child)| {
        let here = (key == app_id).then(|| {
            child
                .get("LaunchOptions")
                .and_then(Vdf::as_str)
                .unwrap_or_default()
                .to_string()
        });
        here.or_else(|| find(child, app_id))
    })
}

/// The newest `localconfig.vdf` among the Steam roots' users.
fn newest_config(roots: &[PathBuf]) -> Option<(PathBuf, SystemTime)> {
    roots
        .iter()
        .flat_map(|root| {
            std::fs::read_dir(root.join("userdata"))
                .into_iter()
                .flatten()
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("config/localconfig.vdf"))
        .filter_map(|p| Some((p.clone(), std::fs::metadata(&p).ok()?.modified().ok()?)))
        .max_by_key(|(_, time)| *time)
}

/// Steam's launch options for `app_id` (cached while the file is unchanged);
/// `None` when Steam's configuration could not be read.
pub fn steam_launch_options(roots: &[PathBuf], app_id: u32) -> Option<String> {
    type Cache = Option<(PathBuf, SystemTime, Option<String>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let (path, time) = newest_config(roots)?;
    let mut cache = CACHE.lock().ok()?;
    if let Some((p, t, value)) = cache.as_ref()
        && *p == path
        && *t == time
    {
        return value.clone();
    }
    let value = read(&path, app_id);
    *cache = Some((path, time, value.clone()));
    value
}

fn read(path: &Path, app_id: u32) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(find(&vdf::parse(&text), &app_id.to_string()).unwrap_or_default())
}

/// Whether `options` make Wine load the native `dll` (`winmm=n,b`, `n`, …).
pub fn enables(options: &str, dll: &str) -> bool {
    let Some(start) = options.find("WINEDLLOVERRIDES=") else {
        return false;
    };
    let value = options[start + "WINEDLLOVERRIDES=".len()..]
        .trim_start_matches(['"', '\''])
        .split(['"', '\'', ' '])
        .next()
        .unwrap_or_default();
    value.split(';').any(|rule| {
        let (names, mode) = rule.split_once('=').unwrap_or((rule, ""));
        names.split(',').any(|n| n.trim().eq_ignore_ascii_case(dll))
            && mode.trim_start().starts_with('n')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_options_are_read_and_checked() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("userdata/42/config");
        std::fs::create_dir_all(&config).unwrap();
        let vdf = r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" {
            "3321460" { "LastPlayed" "1" "LaunchOptions" "WINEDLLOVERRIDES=\"winmm=n,b\" %command%" }
            "570" { "LastPlayed" "2" } } } } } }"#;
        std::fs::write(config.join("localconfig.vdf"), vdf).unwrap();
        let roots = [root.path().to_path_buf()];
        let options = steam_launch_options(&roots, 3_321_460).unwrap();
        assert!(enables(&options, "winmm"));
        assert!(!enables(&options, "version"));
        assert_eq!(
            read(&config.join("localconfig.vdf"), 570).as_deref(),
            Some("")
        );
        assert!(enables(
            "WINEDLLOVERRIDES='dxgi,winmm=n' %command%",
            "winmm"
        ));
        assert!(!enables("WINEDLLOVERRIDES=\"winmm=b\" %command%", "winmm"));
        assert!(!enables("%command% -dx12", "winmm"));
    }
}
