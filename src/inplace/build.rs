//! Building and deploying in-place archives and index redirections (D-14).
//!
//! Patched files reach the engine through the index (`0.pamt`) of the group
//! holding the vanilla file. Large vanilla `.paz` archives are never touched;
//! Pailune packs new entries into a symlinked `<k>.paz` for each affected group,
//! redirects records in `0.pamt`, and updates `0.papgt`.
//!
//! A base backup of all vanilla index files is created on first run, and every
//! change is stored as a versioned snapshot for safe rollback and restore.

use super::classify::is_inplace_candidate;
use super::store::{Fingerprint, Snapshot, Store, Stored, sha256_file, write_atomic};
use crate::error::{IoContext, PailuneError, Result};
use crate::mod_engine::plan::LinkOp;
use crate::paz::hash::pa_hash;
use crate::paz::pamt;
use crate::paz::pamt_edit::{self, AddedPaz, Entry};
use crate::paz::papgt::{self, Papgt};
use crate::paz::{encode, vanilla::Vanilla};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// What an in-place build produces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InPlacePlan {
    /// Symlinks to create for added archives (`<group>/<k>.paz`).
    pub links: Vec<LinkOp>,
    /// Number of files successfully redirected.
    pub file_count: usize,
    /// Names of affected vanilla groups (e.g. `["0008"]`).
    pub groups: Vec<String>,
    /// The updated `Papgt` containing new `pamt_crc`s for modified groups.
    pub papgt: Papgt,
    /// Files skipped with a reason.
    pub skipped: Vec<String>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_paver_hex(meta_dir: &Path) -> String {
    let paver = meta_dir.join("0.paver");
    fs::read(&paver).map_or_else(|_| String::new(), |b| hex(&b))
}

/// Ensures the base backup (0) exists in `store`.
pub fn ensure_base_backup(
    game_dir: &Path,
    store: &Store,
    meta_dir: &Path,
    vanilla: &Vanilla,
    now: i64,
) -> Result<()> {
    if store.has_base() {
        let base = store.manifest(0)?;
        let current_paver = read_paver_hex(meta_dir);
        if !base.game_build.is_empty()
            && !current_paver.is_empty()
            && base.game_build != current_paver
        {
            return Err(PailuneError::Invalid(
                "game version changed since in-place base backup was taken — unlink, \
                 verify files in Steam, and refresh backup before applying"
                    .into(),
            ));
        }
        let live = store.live_state(game_dir)?;
        if live.is_none() {
            return Err(PailuneError::Invalid(
                "game index files do not match base or any version; \
                 they may have been modified by a game update or external tool"
                    .into(),
            ));
        }
        if live != Some(0) {
            store.restore(0, game_dir)?;
        }
        return Ok(());
    }

    // Creating initial base backup
    let game_build = read_paver_hex(meta_dir);
    let mut base_files: Vec<(String, Vec<u8>)> = Vec::new();

    let papgt_path = meta_dir.join("0.papgt");
    let papgt_bytes = match fs::read(&papgt_path) {
        Ok(b) => b,
        Err(_) => papgt::serialize(&vanilla.papgt)?,
    };
    base_files.push(("meta/0.papgt".to_string(), papgt_bytes));

    let pathc_path = meta_dir.join("0.pathc");
    if let Ok(pathc_bytes) = fs::read(&pathc_path) {
        base_files.push(("meta/0.pathc".to_string(), pathc_bytes));
    }

    for group in &vanilla.papgt.groups {
        let pamt_path = game_dir.join(&group.name).join("0.pamt");
        if let Ok(bytes) = fs::read(&pamt_path) {
            base_files.push((format!("{}/0.pamt", group.name), bytes));
        }
    }

    let mut archives = Vec::new();
    for group in &vanilla.papgt.groups {
        let group_dir = game_dir.join(&group.name);
        if let Ok(entries) = fs::read_dir(&group_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "paz") {
                    let rel = format!(
                        "{}/{}",
                        group.name,
                        entry.file_name().to_string_lossy()
                    );
                    let size = entry.metadata().map_or(0, |m| m.len());
                    let sha256 = sha256_file(&path).unwrap_or_default();
                    archives.push(Fingerprint { rel, size, sha256 });
                }
            }
        }
    }

    let snap = Snapshot {
        label: "vanilla base".into(),
        game_build,
        files: base_files,
        archives,
        links: Vec::new(),
    };
    store.save(true, snap, now)?;
    Ok(())
}

/// Builds in-place archives and rewrites `0.pamt` index files for candidate files.
pub fn build(
    game_dir: &Path,
    store: &Store,
    meta_dir: &Path,
    vanilla: &Vanilla,
    files: &BTreeMap<String, PathBuf>,
    now: i64,
) -> Result<InPlacePlan> {
    if files.is_empty() {
        return Ok(InPlacePlan {
            papgt: vanilla.papgt.clone(),
            ..Default::default()
        });
    }

    ensure_base_backup(game_dir, store, meta_dir, vanilla, now)?;
    let base_manifest = store.manifest(0)?;

    let mut grouped: BTreeMap<String, Vec<(String, PathBuf, u16)>> = BTreeMap::new();
    let mut skipped: Vec<String> = Vec::new();

    for (vpath, source) in files {
        if !is_inplace_candidate(vpath) {
            continue;
        }
        if let Some((group_name, record)) = vanilla.locate(vpath) {
            grouped
                .entry(group_name.clone())
                .or_default()
                .push((vpath.clone(), source.clone(), record.flags));
        } else {
            skipped.push(format!(
                "{vpath}: in-place candidate not found in vanilla archives (cannot redirect)"
            ));
        }
    }

    if grouped.is_empty() {
        return Ok(InPlacePlan {
            papgt: vanilla.papgt.clone(),
            skipped,
            ..Default::default()
        });
    }

    let mut links = Vec::new();
    let mut version_links = Vec::new();
    let mut version_files = Vec::new();
    let mut updated_papgt = vanilla.papgt.clone();
    let mut affected_groups = Vec::new();
    let mut total_files = 0;

    for (group_name, mut entries) in grouped {
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let base_pamt = store.read(&base_manifest, &format!("{group_name}/0.pamt"))?;

        let mut paz_bytes = Vec::new();
        let mut redirects = BTreeMap::new();

        for (vpath, source, flags) in entries {
            let raw_data = fs::read(&source).at(&source)?;
            let stored = encode::encode(&vpath, &raw_data, flags)?;
            let offset = paz_bytes.len() as u32;
            paz_bytes.extend(&stored);
            paz_bytes.resize(paz_bytes.len().next_multiple_of(16), 0);
            redirects.insert(
                vpath.clone(),
                Entry {
                    offset,
                    stored_size: stored.len() as u32,
                    orig_size: raw_data.len() as u32,
                    flags,
                },
            );
            total_files += 1;
        }

        let paz_hash = store.put_archive(&paz_bytes)?;
        let added = AddedPaz {
            crc: pa_hash(&paz_bytes),
            size: paz_bytes.len() as u32,
        };

        let (new_pamt, archive_index) = pamt_edit::redirect(&base_pamt, added, &redirects)?;

        // Self-verification: confirm rewritten index parses and contains redirected entries
        let parsed = pamt::parse(&new_pamt)?;
        for (vpath, expected) in &redirects {
            let record = parsed
                .files
                .get(vpath)
                .ok_or_else(|| PailuneError::Archive(format!("verification failed: {vpath} missing")))?;
            if (record.offset, record.stored_size, record.orig_size, record.flags, record.paz_index as u32)
                != (expected.offset, expected.stored_size, expected.orig_size, expected.flags, archive_index)
            {
                return Err(PailuneError::Archive(format!(
                    "verification failed: record mismatch for {vpath}"
                )));
            }
        }

        let paz_rel = format!("{group_name}/{archive_index}.paz");
        let paz_target = game_dir.join(&paz_rel);
        let paz_source = store.archive_path(&paz_hash);
        links.push(LinkOp {
            source: paz_source,
            target: paz_target,
        });
        version_links.push(Stored {
            rel: paz_rel,
            sha256: paz_hash,
        });

        let new_pamt_crc = pa_hash(&new_pamt[12..]);
        version_files.push((format!("{group_name}/0.pamt"), new_pamt));

        if let Some(g) = updated_papgt.groups.iter_mut().find(|g| g.name == group_name) {
            g.pamt_crc = new_pamt_crc;
        }
        affected_groups.push(group_name);
    }

    let updated_papgt_bytes = papgt::serialize(&updated_papgt)?;
    version_files.push(("meta/0.papgt".to_string(), updated_papgt_bytes));

    // Preserve any unchanged index files from base
    for f in &base_manifest.files {
        if !version_files.iter().any(|(rel, _)| rel == &f.rel) {
            let bytes = store.read(&base_manifest, &f.rel)?;
            version_files.push((f.rel.clone(), bytes));
        }
    }

    let snap = Snapshot {
        label: format!(
            "in-place apply ({} files, {} groups)",
            total_files,
            affected_groups.len()
        ),
        game_build: base_manifest.game_build,
        files: version_files.clone(),
        archives: Vec::new(),
        links: version_links,
    };
    store.save(false, snap, now)?;

    for (rel, bytes) in &version_files {
        write_atomic(&game_dir.join(rel), bytes)?;
    }

    Ok(InPlacePlan {
        links,
        file_count: total_files,
        groups: affected_groups,
        papgt: updated_papgt,
        skipped,
    })
}

/// Restores vanilla base index files if an in-place version was active.
pub fn revert(game_dir: &Path, store: &Store) -> Result<()> {
    if store.has_base() {
        if let Ok(Some(n)) = store.live_state(game_dir) {
            if n > 0 {
                store.restore(0, game_dir)?;
            }
        }
    }
    Ok(())
}
