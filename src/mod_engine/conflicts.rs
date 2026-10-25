//! Per-mod file conflict analysis for the enabled mods.

use super::model::Registry;
use super::plan::{Placement, deployable_files, mod_ops, virtual_path};
use crate::config::Config;
use crate::error::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One contested game file and the other mod involved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contest {
    /// Target path relative to the game directory.
    pub file: PathBuf,
    /// The mod this one wins against or loses to.
    pub other: String,
}

/// Conflicts of a single mod.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModConflicts {
    /// Files this mod provides that later mods overwrite.
    pub loses: Vec<Contest>,
    /// Files this mod overwrites from earlier mods.
    pub wins: Vec<Contest>,
}

/// Conflicts for every enabled mod that has any.
pub type ConflictReport = BTreeMap<String, ModConflicts>;

/// Shown in front of archive paths in conflict lists.
pub const ARCHIVE_PREFIX: &str = "[archive]";

/// Analyses which enabled mods provide the same game files.
pub fn analyze(config: &Config, staging: &Path, registry: &Registry) -> Result<ConflictReport> {
    // Linked files are keyed by their game path, packed files by their
    // archive path, so both kinds of override show up.
    let game = &config.game.dir;
    let provided = |m: &crate::mod_engine::model::ModEntry| -> Result<Vec<PathBuf>> {
        let links = mod_ops(config, staging, &m.name, m.variant.as_deref())?
            .into_iter()
            .map(|op| op.target);
        let packed = deployable_files(&staging.join(&m.name), m.variant.as_deref())?
            .into_iter()
            .filter(|f| f.place == Placement::Pack)
            .filter_map(|f| virtual_path(&f.rel))
            .map(|v| game.join(ARCHIVE_PREFIX).join(v));
        Ok(links.chain(packed).collect())
    };
    let providers: BTreeMap<PathBuf, Vec<String>> = registry
        .mods
        .iter()
        .filter(|m| m.enabled)
        .map(|m| provided(m).map(|targets| (m.name.clone(), targets)))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flat_map(|(name, targets)| targets.into_iter().map(move |t| (t, name.clone())))
        .fold(BTreeMap::new(), |mut acc, (target, name)| {
            acc.entry(target).or_insert_with(Vec::new).push(name);
            acc
        });
    Ok(providers
        .into_iter()
        .filter(|(_, mods)| mods.len() > 1)
        .flat_map(|(target, mods)| {
            let file = target.strip_prefix(game).unwrap_or(&target).to_path_buf();
            let winner = mods.last().cloned().unwrap_or_default();
            let losers = mods[..mods.len() - 1].to_vec();
            losers.into_iter().flat_map(move |loser| {
                [
                    (
                        loser.clone(),
                        false,
                        Contest {
                            file: file.clone(),
                            other: winner.clone(),
                        },
                    ),
                    (
                        winner.clone(),
                        true,
                        Contest {
                            file: file.clone(),
                            other: loser,
                        },
                    ),
                ]
            })
        })
        .fold(ConflictReport::new(), |mut report, (name, won, contest)| {
            let entry = report.entry(name).or_default();
            match won {
                true => entry.wins.push(contest),
                false => entry.loses.push(contest),
            }
            report
        }))
}
