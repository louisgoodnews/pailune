//! Building Pylune's group and the rebuilt `0.papgt` in the data folder.

use super::{OverlayInput, OverlayPlan};
use crate::error::{IoContext, PyluneError, Result};
use crate::mod_engine::plan::LinkOp;
use crate::paz::write::{self, Packed};
use crate::paz::{encode, pamt, papgt, vanilla};
use crate::steam::SteamInfo;
use crate::symlink::bak_path;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Overlay group numbers start here, far above the game's own groups.
const FIRST_GROUP: u32 = 100;

/// `count` unused four-digit group names above every vanilla number.
fn free_names(game_dir: &Path, taken: &BTreeSet<String>, count: usize) -> Vec<String> {
    let start = taken
        .iter()
        .filter_map(|n| n.parse::<u32>().ok())
        .max()
        .map_or(FIRST_GROUP, |m| (m + 1).max(FIRST_GROUP));
    (start..)
        .map(|n| format!("{n:04}"))
        .filter(|name| !taken.contains(name) && !game_dir.join(name).exists())
        .take(count)
        .collect()
}

/// Encodes each packable file with the flags of the game file it replaces.
fn pack(input: &OverlayInput, base: &vanilla::Vanilla) -> Result<(Vec<Packed>, Vec<String>)> {
    input.packs.iter().try_fold(
        (Vec::new(), Vec::new()),
        |(mut packed, mut skipped), (vpath, source)| {
            let data = fs::read(source).at(source)?;
            let flags = base.flags_for(vpath);
            match encode::encode(vpath, &data, flags) {
                Ok(stored) => packed.push(Packed {
                    vpath: vpath.clone(),
                    stored,
                    orig_size: data.len() as u32,
                    flags,
                }),
                Err(err) => skipped.push(err.to_string()),
            }
            Ok((packed, skipped))
        },
    )
}

/// Writes `bytes` to `path`, creating the parent folder.
fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).at(dir)?;
    }
    fs::write(path, bytes).at(path)
}

/// Checksum of a prebuilt group's index (validated by parsing it).
fn prebuilt_crc(group: &Path) -> Result<u32> {
    let path = group.join("0.pamt");
    pamt::parse(&fs::read(&path).at(&path)?)
        .map(|index| index.crc)
        .map_err(|err| PyluneError::Invalid(format!("{}: {err}", group.display())))
}

/// Builds the overlay into `out_dir` and returns the links that deploy it.
/// Requires the vanilla `meta/0.papgt.bak` snapshot; nothing in the game
/// directory is touched here.
pub fn build(info: &SteamInfo, input: &OverlayInput, out_dir: &Path) -> Result<OverlayPlan> {
    let snapshot = bak_path(&info.meta_dir.join("0.papgt"));
    let vanilla_bytes = fs::read(&snapshot).map_err(|_| {
        PyluneError::Invalid("no vanilla metadata backup (meta/0.papgt.bak) yet".into())
    })?;
    build_from(info, &vanilla_bytes, input, out_dir)
}

/// [`build`] with explicit vanilla `0.papgt` bytes (used for dry runs).
pub fn build_from(
    info: &SteamInfo,
    vanilla_bytes: &[u8],
    input: &OverlayInput,
    out_dir: &Path,
) -> Result<OverlayPlan> {
    let wanted: BTreeSet<String> = input.packs.keys().cloned().collect();
    let base = vanilla::load(&info.game_dir, vanilla_bytes, &wanted)?;
    let _ = fs::remove_dir_all(out_dir);
    let own = !input.packs.is_empty();
    let names = free_names(
        &info.game_dir,
        &base.group_names(),
        usize::from(own) + input.prebuilt.len(),
    );
    let (packed, skipped) = match own {
        true => pack(input, &base)?,
        false => (Vec::new(), Vec::new()),
    };
    let replaced = packed.iter().filter(|p| base.replaces(&p.vpath)).count();
    let packed_count = packed.len();
    // (name, crc, folder to link) — own group first, then prebuilt groups,
    // later mods before earlier ones (higher priority).
    let mut groups: Vec<(String, u32, PathBuf)> = Vec::new();
    let mut names = names.into_iter();
    if own {
        let group = write::build(packed);
        let name = names.next().unwrap_or_default();
        let dir = out_dir.join(&name);
        write_file(&dir.join("0.paz"), &group.paz)?;
        write_file(&dir.join("0.pamt"), &group.pamt)?;
        fs::write(dir.join(".pylune_owned"), b"").at(&dir)?;
        groups.push((name, group.pamt_crc, dir));
    }
    input
        .prebuilt
        .iter()
        .rev()
        .zip(names)
        .try_for_each(|(folder, name)| {
            groups.push((name, prebuilt_crc(folder)?, folder.clone()));
            Ok::<(), PyluneError>(())
        })?;
    let entries: Vec<(String, u32)> = groups.iter().map(|(n, c, _)| (n.clone(), *c)).collect();
    let table = papgt::serialize(&vanilla::register(&base.papgt, &entries))?;
    let papgt_path = out_dir.join("meta/0.papgt");
    write_file(&papgt_path, &table)?;
    let links = groups
        .iter()
        .map(|(name, _, dir)| LinkOp {
            source: dir.clone(),
            target: info.game_dir.join(name),
        })
        .chain(std::iter::once(LinkOp {
            source: papgt_path,
            target: info.meta_dir.join("0.papgt"),
        }))
        .collect();
    Ok(OverlayPlan {
        links,
        packed: packed_count,
        replaced,
        groups: groups.into_iter().map(|(n, _, _)| n).collect(),
        skipped,
    })
}
