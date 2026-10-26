//! Building Pylune's group and the rebuilt `0.papgt` in the data folder.

use super::{OverlayInput, OverlayPlan};
use crate::error::{IoContext, PyluneError, Result};
use crate::mod_engine::plan::LinkOp;
use crate::paz::pathc::{self, Pathc};
use crate::paz::write::{self, Packed};
use crate::paz::{encode, pamt, papgt, vanilla};
use crate::steam::SteamInfo;
use crate::symlink::bak_path;
use std::collections::{BTreeMap, BTreeSet};
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

/// Encodes each packable file with the flags of the game file it replaces;
/// textures are also registered in `textures` (the PATHC being rebuilt).
fn pack(
    input: &OverlayInput,
    patched: &BTreeMap<String, Vec<u8>>,
    base: &vanilla::Vanilla,
    textures: &mut Pathc,
) -> Result<(Vec<Packed>, Vec<String>)> {
    // Patched content replaces a mod's shipped file of the same path.
    let vpaths: BTreeSet<&String> = input.packs.keys().chain(patched.keys()).collect();
    vpaths.into_iter().try_fold(
        (Vec::new(), Vec::new()),
        |(mut packed, mut skipped), vpath| {
            let data = match (patched.get(vpath), input.packs.get(vpath)) {
                (Some(bytes), _) => bytes.clone(),
                (None, Some(source)) => fs::read(source).at(source)?,
                (None, None) => return Ok((packed, skipped)),
            };
            let flags = super::storage::storage_flags(vpath, base.flags_for(vpath));
            let result = match is_texture(vpath) {
                true => super::textures::pack(vpath, &data, flags, textures),
                false => encode::encode(vpath, &data, flags).map(|stored| Packed {
                    vpath: vpath.clone(),
                    stored,
                    orig_size: data.len() as u32,
                    flags,
                }),
            };
            match result {
                Ok(file) => packed.push(file),
                Err(err) => skipped.push(format!("{vpath}: {err}")),
            }
            Ok((packed, skipped))
        },
    )
}

/// Whether a virtual path is a DDS texture.
fn is_texture(vpath: &str) -> bool {
    vpath.ends_with(".dds")
}

/// Reads a vanilla `.bak` snapshot of a metadata file.
fn snapshot(info: &SteamInfo, name: &str) -> Result<Vec<u8>> {
    let path = bak_path(&info.meta_dir.join(name));
    fs::read(&path).map_err(|_| {
        PyluneError::Invalid(format!("no vanilla metadata backup (meta/{name}.bak) yet"))
    })
}

/// The vanilla `0.papgt`: the snapshot once Pylune has deployed, else the
/// live file (still untouched then).
pub(crate) fn vanilla_papgt(info: &SteamInfo) -> Result<Vec<u8>> {
    snapshot(info, "0.papgt").or_else(|_| {
        let live = info.meta_dir.join("0.papgt");
        fs::read(&live).at(&live)
    })
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
    let textures = input.packs.keys().any(|v| is_texture(v));
    let pathc = match textures {
        true => Some(snapshot(info, "0.pathc")?),
        false => None,
    };
    build_from(
        info,
        &snapshot(info, "0.papgt")?,
        pathc.as_deref(),
        input,
        out_dir,
    )
}

/// [`build`] with explicit vanilla `0.papgt` bytes (used for dry runs).
pub fn build_from(
    info: &SteamInfo,
    vanilla_bytes: &[u8],
    vanilla_pathc: Option<&[u8]>,
    input: &OverlayInput,
    out_dir: &Path,
) -> Result<OverlayPlan> {
    let patch_mods = super::patches::read_all(&input.patches);
    let mut wanted = super::patches::wanted(&patch_mods);
    wanted.paths.extend(input.packs.keys().cloned());
    wanted.paths.extend(
        input
            .text_patches
            .iter()
            .map(|(_, target, _)| target.clone()),
    );
    let base = vanilla::load_cached(&info.game_dir, vanilla_bytes, &wanted)?;
    let _ = fs::remove_dir_all(out_dir);
    let (mut patched, mut patch_notes) =
        super::patches::apply_all(&info.game_dir, &base, &patch_mods, &input.packs);
    super::patches::apply_text(
        &info.game_dir,
        &base,
        &input.text_patches,
        &input.packs,
        &mut patched,
        &mut patch_notes,
    );
    let own = !input.packs.is_empty() || !patched.is_empty();
    let names = free_names(
        &info.game_dir,
        &base.group_names(),
        usize::from(own) + input.prebuilt.len(),
    );
    let mut textures = match vanilla_pathc {
        Some(bytes) => pathc::parse(bytes)?,
        None => Pathc::empty(),
    };
    let (packed, skipped) = match own {
        true => pack(input, &patched, &base, &mut textures)?,
        false => (Vec::new(), Vec::new()),
    };
    let texture_count = packed.iter().filter(|p| is_texture(&p.vpath)).count();
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
        // The game's own indexes keep these invariants; never deploy one that doesn't.
        let problems = crate::paz::pamt_check::problems(&group.pamt, &dir)?;
        if !problems.is_empty() {
            return Err(PyluneError::Archive(format!(
                "the built archive index failed its check: {}",
                problems.join("; ")
            )));
        }
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
    // A rebuilt texture table is deployed only when textures were packed.
    let pathc_link = match (texture_count, vanilla_pathc) {
        (0, _) | (_, None) => None,
        (_, Some(_)) => {
            let path = out_dir.join("meta/0.pathc");
            write_file(&path, &pathc::serialize(&textures))?;
            Some(LinkOp {
                source: path,
                target: info.meta_dir.join("0.pathc"),
            })
        }
    };
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
        .chain(pathc_link)
        .collect();
    Ok(OverlayPlan {
        links,
        packed: packed_count,
        replaced,
        groups: groups.into_iter().map(|(n, _, _)| n).collect(),
        skipped,
        patch_notes,
    })
}
