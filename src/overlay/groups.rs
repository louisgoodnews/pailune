//! Which archive groups Pylune builds: every patched data table (body and
//! its index) in a group of its own, as DMM does (`docs/research/
//! dmm-analysis.md`), and everything else in one main group.

use crate::error::{IoContext, PyluneError, Result};
use crate::paz::write::{self, Packed};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Extensions of data-table files; a table is its path without them.
const TABLE: [&str; 6] = [
    ".staticinfobody",
    ".staticinfoheader",
    ".pabgb",
    ".pabgh",
    ".paseq",
    ".paseqc",
];

/// The table a virtual path belongs to, if it is a data-table file.
pub fn table_of(vpath: &str) -> Option<&str> {
    TABLE.iter().find_map(|ext| vpath.strip_suffix(ext))
}

/// The files of each group: the main group first (if it has files), then
/// one group per table, by table path.
pub fn split(packed: Vec<Packed>) -> Vec<Vec<Packed>> {
    let mut main = Vec::new();
    let mut tables: BTreeMap<String, Vec<Packed>> = BTreeMap::new();
    packed.into_iter().for_each(|p| match table_of(&p.vpath) {
        Some(table) => tables.entry(table.to_string()).or_default().push(p),
        None => main.push(p),
    });
    std::iter::once(main)
        .filter(|m| !m.is_empty())
        .chain(tables.into_values())
        .collect()
}

/// Writes one group into `dir` and returns its index checksum. The index
/// must pass the checks the game's own indexes pass.
pub fn write_group(files: Vec<Packed>, dir: &Path) -> Result<u32> {
    let group = write::build(files);
    fs::create_dir_all(dir).at(dir)?;
    fs::write(dir.join("0.paz"), &group.paz).at(dir)?;
    fs::write(dir.join("0.pamt"), &group.pamt).at(dir)?;
    let problems = crate::paz::pamt_check::problems(&group.pamt, dir)?;
    if !problems.is_empty() {
        return Err(PyluneError::Archive(format!(
            "the built archive index failed its check: {}",
            problems.join("; ")
        )));
    }
    fs::write(dir.join(".pylune_owned"), b"").at(dir)?;
    Ok(group.pamt_crc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_get_groups_of_their_own() {
        let file = |vpath: &str| Packed {
            vpath: vpath.into(),
            stored: vec![1],
            orig_size: 1,
            flags: 0,
        };
        let sets = split(vec![
            file("gamedata/binarystaticinfo__/bin/iteminfo.staticinfobody"),
            file("ui/a.css"),
            file("gamedata/binarystaticinfo__/bin/dropsetinfo.staticinfoheader"),
            file("gamedata/binarystaticinfo__/bin/iteminfo.staticinfoheader"),
            file("gamedata/binarystaticinfo__/bin/dropsetinfo.staticinfobody"),
        ]);
        let names: Vec<Vec<&str>> = sets
            .iter()
            .map(|s| {
                s.iter()
                    .map(|p| p.vpath.rsplit('/').next().unwrap())
                    .collect()
            })
            .collect();
        assert_eq!(
            names,
            [
                vec!["a.css"],
                vec!["dropsetinfo.staticinfoheader", "dropsetinfo.staticinfobody"],
                vec!["iteminfo.staticinfobody", "iteminfo.staticinfoheader"],
            ]
        );
        assert!(
            split(vec![file("gamedata/x/y.pabgb")]).len() == 1,
            "no empty main group"
        );
    }
}
