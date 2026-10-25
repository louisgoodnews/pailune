//! A read-only summary of the installed archives for the UI.

use super::{papgt, paver};
use std::fs;
use std::path::Path;

/// What the game's archive metadata says about the installation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiveStatus {
    /// e.g. "2.3.2"; `None` if `0.paver` is missing or invalid.
    pub version: Option<String>,
    /// Pack groups listed in `meta/0.papgt`.
    pub registered: usize,
    /// Archive folders on disk (with a `0.pamt`) that the game does not load,
    /// e.g. leftovers of other mod managers after a game update.
    pub unregistered: Vec<String>,
    /// Problem reading the metadata, if any.
    pub error: Option<String>,
}

/// Inspects `meta/0.paver`, `meta/0.papgt` and the group folders.
pub fn inspect(game_dir: &Path) -> ArchiveStatus {
    let version = fs::read(game_dir.join("meta/0.paver"))
        .ok()
        .and_then(|d| paver::parse(&d).ok())
        .map(|v| v.to_string());
    let table = fs::read(game_dir.join("meta/0.papgt"))
        .map_err(|e| e.to_string())
        .and_then(|d| papgt::parse(&d).map_err(|e| e.to_string()));
    let Ok(table) = table else {
        return ArchiveStatus {
            version,
            error: table.err(),
            ..ArchiveStatus::default()
        };
    };
    let mut unregistered: Vec<String> = fs::read_dir(game_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().join("0.pamt").is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|name| table.groups.iter().all(|g| &g.name != name))
                .collect()
        })
        .unwrap_or_default();
    unregistered.sort();
    ArchiveStatus {
        version,
        registered: table.groups.len(),
        unregistered,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paz::papgt::{GroupEntry, Papgt, serialize};

    #[test]
    fn reports_inactive_overlay_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path();
        let table = Papgt {
            unknown0: 1,
            unknown1: 2,
            groups: vec![GroupEntry {
                is_optional: false,
                lang_mask: 0x7FFF,
                name: "0000".into(),
                pamt_crc: 0,
            }],
        };
        fs::create_dir_all(game.join("meta")).unwrap();
        fs::write(game.join("meta/0.papgt"), serialize(&table).unwrap()).unwrap();
        fs::write(
            game.join("meta/0.paver"),
            [2, 0, 3, 0, 2, 0, 0xE3, 0xCC, 0x19, 0xCB],
        )
        .unwrap();
        ["0000", "dmmsa"].iter().for_each(|g| {
            fs::create_dir_all(game.join(g)).unwrap();
            fs::write(game.join(g).join("0.pamt"), b"x").unwrap();
        });
        let status = inspect(game);
        assert_eq!(status.version.as_deref(), Some("2.3.2"));
        assert_eq!(
            (status.registered, status.unregistered),
            (1, vec!["dmmsa".to_string()])
        );
    }
}
