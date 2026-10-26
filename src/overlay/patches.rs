//! Applying byte-patch JSON mods to vanilla game data for the overlay.

use crate::error::{IoContext, PyluneError, Result};
use crate::patch::{self, JsonMod, apply::apply};
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

/// Old and new extensions of table files (renamed in a game update).
const ALIASES: [(&str, &str); 2] = [
    (".pabgb", ".staticinfobody"),
    (".pabgh", ".staticinfoheader"),
];

/// The path plus its renamed-extension alias.
fn variants(path: &str) -> Vec<String> {
    let lower = path.to_lowercase().replace('\\', "/");
    let swapped = ALIASES.iter().find_map(|(a, b)| {
        lower
            .strip_suffix(a)
            .map(|s| format!("{s}{b}"))
            .or_else(|| lower.strip_suffix(b).map(|s| format!("{s}{a}")))
    });
    std::iter::once(lower).chain(swapped).collect()
}

/// The table index belonging to a table body, if `vpath` is one.
fn header_of(vpath: &str) -> Option<String> {
    [
        (".pabgb", ".pabgh"),
        (".staticinfobody", ".staticinfoheader"),
    ]
    .iter()
    .find_map(|(body, head)| vpath.strip_suffix(body).map(|s| format!("{s}{head}")))
}

/// File name of a virtual path.
fn name_of(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
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
fn resolve(base: &Vanilla, game_file: &str) -> Result<String> {
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
            PyluneError::Invalid(format!("game file {game_file:?} not found (or ambiguous)"))
        })
}

/// Vanilla plaintext of a located file.
fn vanilla_bytes(game_dir: &Path, base: &Vanilla, vpath: &str) -> Result<Vec<u8>> {
    let (group, record) = base
        .locate(vpath)
        .ok_or_else(|| PyluneError::Invalid(format!("{vpath} not in the game")))?;
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
                    None => match packed.get(&vpath) {
                        Some(source) => fs::read(source).at(source)?,
                        None => vanilla_bytes(game_dir, base, &vpath)?,
                    },
                };
                let header = header_of(&vpath)
                    .map(|h| vanilla_bytes(game_dir, base, &h))
                    .transpose()?;
                Ok((vpath, current, header))
            });
            match target {
                Err(err) => note(&mut notes, m, false, format!("{file_name}: {err}")),
                Ok((vpath, mut data, header)) => {
                    let report = apply(
                        &mut data,
                        p,
                        header.as_deref(),
                        header.is_some(),
                        parsed.allow_partial,
                    );
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
            let base_text = String::from_utf8_lossy(&base_bytes);
            let name = file.file_name().unwrap_or_default().to_string_lossy();
            match target.ends_with(".xml") {
                false => Ok((
                    patch::css::apply(&base_text, &patch),
                    "CSS patch applied".to_string(),
                )),
                true => patch::xml::apply(&base_text, &name, &patch)
                    .map(|out| {
                        let missed = match out.missed.len() {
                            0 => String::new(),
                            n => format!("; {n} target(s) not found: {}", out.missed.join(", ")),
                        };
                        (
                            out.text,
                            format!("XML patch applied ({} change(s)){missed}", out.applied),
                        )
                    })
                    .map_err(PyluneError::Invalid),
            }
        });
        let (ok, message) = match result {
            Ok((text, what)) => {
                files.insert(target.clone(), text.into_bytes());
                (true, format!("{target}: {what}"))
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
