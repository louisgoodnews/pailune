//! Finding the game through Steam's own library configuration.

use super::vdf::{self, Vdf};
use std::fs;
use std::path::{Path, PathBuf};

/// Where Steam keeps its data for native, Debian-style and Flatpak installs.
fn steam_roots() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut roots: Vec<PathBuf> = [
        ".local/share/Steam",
        ".steam/steam",
        ".steam/root",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam",
        "snap/steam/common/.local/share/Steam",
    ]
    .iter()
    .map(|rel| home.join(rel))
    .filter_map(|p| fs::canonicalize(p).ok())
    .collect();
    roots.dedup();
    roots
}

/// Library folders listed in `<root>/steamapps/libraryfolders.vdf`.
fn libraries(root: &Path) -> Vec<PathBuf> {
    let file = root.join("steamapps/libraryfolders.vdf");
    let listed: Vec<PathBuf> = fs::read_to_string(file)
        .map(|text| vdf::parse(&text))
        .ok()
        .and_then(|doc| doc.get("libraryfolders").cloned())
        .map(|folders| {
            folders
                .entries()
                .iter()
                .filter_map(|(_, f)| f.get("path").and_then(Vdf::as_str).map(PathBuf::from))
                .collect()
        })
        .unwrap_or_default();
    std::iter::once(root.to_path_buf()).chain(listed).collect()
}

/// The install directory named by `appmanifest_<id>.acf` in a library.
fn installed_in(library: &Path, app_id: u32) -> Option<PathBuf> {
    let manifest = library.join(format!("steamapps/appmanifest_{app_id}.acf"));
    let doc = vdf::parse(&fs::read_to_string(manifest).ok()?);
    let dir = doc.get("AppState")?.get("installdir")?.as_str()?;
    Some(library.join("steamapps/common").join(dir)).filter(|p| p.is_dir())
}

/// Searches every Steam library for the game; returns its install directory.
pub fn find_game(app_id: u32) -> Option<PathBuf> {
    find_app(app_id)
}

/// Install directory of any Steam app or tool (e.g. a Steam Linux Runtime).
pub fn find_app(app_id: u32) -> Option<PathBuf> {
    steam_roots()
        .iter()
        .flat_map(|root| libraries(root))
        .find_map(|library| installed_in(&library, app_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_game_in_a_secondary_library() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Steam");
        let lib = tmp.path().join("games");
        fs::create_dir_all(root.join("steamapps")).unwrap();
        fs::create_dir_all(lib.join("steamapps/common/Crimson Desert")).unwrap();
        let folders = format!(
            "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}",
            root.display(),
            lib.display()
        );
        fs::write(root.join("steamapps/libraryfolders.vdf"), folders).unwrap();
        fs::write(
            lib.join("steamapps/appmanifest_3321460.acf"),
            "\"AppState\" { \"appid\" \"3321460\" \"installdir\" \"Crimson Desert\" }",
        )
        .unwrap();
        let found = libraries(&root)
            .iter()
            .find_map(|l| installed_in(l, 3_321_460));
        assert_eq!(found, Some(lib.join("steamapps/common/Crimson Desert")));
        assert_eq!(
            libraries(&root).iter().find_map(|l| installed_in(l, 1)),
            None
        );
    }
}

#[cfg(test)]
mod machine {
    /// Run with `cargo test -- --ignored` on a machine with the game installed.
    #[test]
    #[ignore = "depends on the local Steam installation"]
    fn detects_the_installed_game() {
        let found = super::find_game(crate::config::CRIMSON_DESERT_APP_ID);
        println!("detected: {found:?}");
        assert!(found.is_some_and(|dir| dir.join("bin64/CrimsonDesert.exe").is_file()));
    }
}
