//! ASI plugins in the game folder (decision D-12): which are there, which
//! mod each belongs to, and their settings files.

use crate::symlink::LinkRecord;
use std::path::{Path, PathBuf};

/// Folders the Ultimate ASI Loader loads plugins from, relative to `bin64/`.
const FOLDERS: [&str; 3] = ["", "scripts", "plugins"];

/// One `.asi` plugin in the game folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    /// Path relative to `bin64/`.
    pub name: String,
    pub path: PathBuf,
    /// The Pylune mod that deployed it; `None` when installed otherwise.
    pub owner: Option<String>,
    /// Its settings file (`<stem>.ini` next to it), if any.
    pub ini: Option<PathBuf>,
}

/// The mod a deployed file comes from: the first folder below staging.
fn owner(path: &Path, links: &[LinkRecord], staging: &Path) -> Option<String> {
    links
        .iter()
        .find(|l| l.target == path)
        .and_then(|l| l.source.strip_prefix(staging).ok())
        .and_then(|rel| rel.components().next())
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
}

/// Every plugin below `bin64`, sorted by name.
pub fn scan(bin64: &Path, links: &[LinkRecord], staging: &Path) -> Vec<Plugin> {
    let mut found: Vec<Plugin> = FOLDERS
        .iter()
        .flat_map(|folder| std::fs::read_dir(bin64.join(folder)).into_iter().flatten())
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("asi")))
        .map(|path| {
            let ini = path.with_extension("ini");
            Plugin {
                name: path
                    .strip_prefix(bin64)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned(),
                owner: owner(&path, links, staging),
                ini: ini.is_file().then_some(ini),
                path,
            }
        })
        .collect();
    found.sort_by_key(|p| p.name.to_lowercase());
    found
}

/// The settings files a staged mod ships (`*.ini`), relative to its root.
pub fn mod_settings(mod_root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = walkdir::WalkDir::new(mod_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("ini"))
        })
        .filter_map(|e| e.path().strip_prefix(mod_root).ok().map(Path::to_path_buf))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_plugins_owners_and_settings() {
        let dir = tempfile::tempdir().unwrap();
        let (bin64, staging) = (dir.path().join("bin64"), dir.path().join("staging"));
        fs::create_dir_all(bin64.join("scripts")).unwrap();
        fs::create_dir_all(staging.join("Skip/bin64")).unwrap();
        fs::write(staging.join("Skip/bin64/skip.asi"), "").unwrap();
        fs::write(staging.join("Skip/bin64/skip.ini"), "[Main]\nSpeed=2\n").unwrap();
        fs::write(bin64.join("skip.asi"), "").unwrap();
        fs::write(bin64.join("skip.ini"), "").unwrap();
        fs::write(bin64.join("scripts/Other.ASI"), "").unwrap();
        let links = [LinkRecord {
            target: bin64.join("skip.asi"),
            source: staging.join("Skip/bin64/skip.asi"),
            backup: None,
            stamp: None,
        }];
        let plugins = scan(&bin64, &links, &staging);
        let summary: Vec<_> = plugins
            .iter()
            .map(|p| (p.name.as_str(), p.owner.as_deref(), p.ini.is_some()))
            .collect();
        assert_eq!(
            summary,
            [
                ("scripts/Other.ASI", None, false),
                ("skip.asi", Some("Skip"), true)
            ]
        );
        assert_eq!(
            mod_settings(&staging.join("Skip")),
            [PathBuf::from("bin64/skip.ini")]
        );
    }
}
