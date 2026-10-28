//! Applying byte-patch JSON mods to vanilla game data for the overlay.

use super::table_paths::{header_of, name_of, variants};
use crate::error::{IoContext, PailuneError, Result};
use crate::patch::{self, JsonMod, apply::apply_over};
use crate::paz::{
    extract,
    vanilla::{Vanilla, Wanted},
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A JSON mod file read from staging (or why it could not be read).
pub struct ModPatch {
    pub mod_name: String,
    pub file: PathBuf,
    pub parsed: Result<JsonMod>,
}

/// One line of the patch report, attributed to a mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchNote {
    pub mod_name: String,
    pub ok: bool,
    pub message: String,
}

/// Reads and parses every JSON patch file, keeping load order.
pub fn read_all(files: &[(String, PathBuf)]) -> Vec<ModPatch> {
    files
        .iter()
        .map(|(mod_name, file)| ModPatch {
            mod_name: mod_name.clone(),
            file: file.clone(),
            parsed: fs::read_to_string(file)
                .at(file)
                .and_then(|t| patch::parse::parse(&t)),
        })
        .collect()
}

/// Files the vanilla scan must locate for these patches.
pub fn wanted(mods: &[ModPatch]) -> Wanted {
    mods.iter()
        .filter_map(|m| m.parsed.as_ref().ok())
        .flat_map(|m| &m.patches)
        .fold(Wanted::default(), |mut w, p| {
            variants(&p.game_file).into_iter().for_each(|v| {
                if let Some(header) = header_of(&v) {
                    w.names.insert(name_of(&header));
                }
                w.names.insert(name_of(&v));
                w.paths.insert(v);
            });
            w
        })
}

/// Resolves a mod's `game_file` to a vanilla virtual path: exact path (or
/// alias) first, then a unique file name.
pub(super) fn resolve(base: &Vanilla, game_file: &str) -> Result<String> {
    let names = variants(game_file);
    names
        .iter()
        .find(|v| base.locate(v).is_some())
        .cloned()
        .or_else(|| {
            let found: Vec<&String> = names
                .iter()
                .flat_map(|v| base.with_name(&name_of(v)))
                .collect();
            (found.len() == 1).then(|| found[0].clone())
        })
        .ok_or_else(|| {
            PailuneError::Invalid(format!("game file {game_file:?} not found (or ambiguous)"))
        })
}

/// Vanilla plaintext of a located file.
pub(super) fn vanilla_bytes(game_dir: &Path, base: &Vanilla, vpath: &str) -> Result<Vec<u8>> {
    let (group, record) = base
        .locate(vpath)
        .ok_or_else(|| PailuneError::Invalid(format!("{vpath} not in the game")))?;
    extract::extract(&game_dir.join(group), vpath, record)
}

/// Applies all patches in load order. Returns patched files (virtual path
/// → plaintext, starting from `packed` content when a mod also ships the
/// file) and a report per mod and file.
pub fn apply_all(
    game_dir: &Path,
    base: &Vanilla,
    mods: &[ModPatch],
    packed: &BTreeMap<String, PathBuf>,
) -> (BTreeMap<String, Vec<u8>>, Vec<PatchNote>) {
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    // What each file looked like before any mod changed it.
    let mut unmodded: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut notes = Vec::new();
    let note = |notes: &mut Vec<PatchNote>, m: &ModPatch, ok, message: String| {
        notes.push(PatchNote {
            mod_name: m.mod_name.clone(),
            ok,
            message,
        });
    };
    mods.iter().for_each(|m| {
        let file_name = m
            .file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parsed = match &m.parsed {
            Ok(parsed) => parsed,
            Err(err) => return note(&mut notes, m, false, format!("{file_name}: {err}")),
        };
        parsed.patches.iter().for_each(|p| {
            let target = resolve(base, &p.game_file).and_then(|vpath| {
                let current = match files.get(&vpath) {
                    Some(bytes) => bytes.clone(),
                    None => {
                        let first = match packed.get(&vpath) {
                            Some(source) => fs::read(source).at(source)?,
                            None => vanilla_bytes(game_dir, base, &vpath)?,
                        };
                        unmodded.insert(vpath.clone(), first.clone());
                        first
                    }
                };
                // The index must come from the same place as the body.
                let header = header_of(&vpath)
                    .map(|h| match (packed.get(&h), packed.contains_key(&vpath)) {
                        (Some(own), _) => fs::read(own).at(own),
                        (None, true) => Err(PailuneError::Invalid(format!(
                            "{vpath}: a mod ships this table without its index ({h})"
                        ))),
                        (None, false) => vanilla_bytes(game_dir, base, &h),
                    })
                    .transpose()?;
                Ok((vpath, current, header))
            });
            match target {
                Err(err) => note(&mut notes, m, false, format!("{file_name}: {err}")),
                Ok((vpath, mut data, header)) => {
                    let (patch, unresolved) =
                        crate::tables::format3::with_fields(p, &vpath, header.as_deref(), &data);
                    // Later mods override earlier ones on the same bytes.
                    let mut report = apply_over(
                        &mut data,
                        &patch,
                        header.as_deref(),
                        header.is_some(),
                        parsed.allow_partial,
                        unmodded.get(&vpath).map(Vec::as_slice),
                    );
                    report.failed.extend(unresolved);
                    let mut header = header;
                    if let Some(rewritten) = report.rewritten_header {
                        header = Some(rewritten);
                    }
                    let ok = report.failed.is_empty();
                    let mut message = format!(
                        "{vpath}: {} applied, {} already applied, {} failed",
                        report.applied,
                        report.already,
                        report.failed.len()
                    );
                    report
                        .failed
                        .iter()
                        .take(5)
                        .for_each(|f| message.push_str(&format!("; {f}")));
                    note(&mut notes, m, ok, message);
                    if report.applied > 0 || report.already > 0 {
                        // The game reads a table's body from the group that
                        // holds its header, so the header goes along.
                        let companion = header_of(&vpath).zip(header);
                        if let Some((path, bytes)) = companion
                            && !packed.contains_key(&path)
                        {
                            files.insert(path, bytes);
                        }
                        files.insert(vpath, data);
                    }
                }
            }
        });
    });
    (files, notes)
}

/// Applies CSS and XML patch/merge files in load order on top of `files`
/// (patched content), the mod's own shipped file, or vanilla.
pub fn apply_text(
    game_dir: &Path,
    base: &Vanilla,
    patches: &[(String, String, PathBuf)],
    packed: &BTreeMap<String, PathBuf>,
    files: &mut BTreeMap<String, Vec<u8>>,
    notes: &mut Vec<PatchNote>,
) {
    patches.iter().for_each(|(mod_name, target, file)| {
        let current = match (files.get(target), packed.get(target)) {
            (Some(bytes), _) => Ok(bytes.clone()),
            (None, Some(source)) => fs::read(source).at(source),
            (None, None) => vanilla_bytes(game_dir, base, target),
        };
        let result = current.and_then(|base_bytes| {
            let patch = fs::read_to_string(file).at(file)?;
            // Never rewrite a file lossily: untouched bytes must stay as shipped.
            let base_text = String::from_utf8(base_bytes).map_err(|_| {
                PailuneError::Invalid("the game file is not UTF-8 text; not patched".into())
            })?;
            let name = file.file_name().unwrap_or_default().to_string_lossy();
            patch::text::apply(target, &name, &base_text, &patch).map(|r| (r, base_text))
        });
        let (ok, message) = match result {
            Ok(((text, what, all_found), before)) => {
                let changed = text != before;
                if changed {
                    files.insert(target.clone(), text.into_bytes());
                }
                let note = if changed { "" } else { " — nothing changed" };
                (all_found, format!("{target}: {what}{note}"))
            }
            Err(err) => (false, format!("{target}: {err}")),
        };
        notes.push(PatchNote {
            mod_name: mod_name.clone(),
            ok,
            message,
        });
    });
}
