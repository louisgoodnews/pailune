//! Per-mod file conflict analysis for the enabled mods.

use super::model::Registry;
use super::plan::mod_ops;
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

/// Analyses which enabled mods provide the same game files.
pub fn analyze(config: &Config, staging: &Path, registry: &Registry) -> Result<ConflictReport> {
    let providers: BTreeMap<PathBuf, Vec<String>> = registry
        .mods
        .iter()
        .filter(|m| m.enabled)
        .map(|m| mod_ops(config, staging, &m.name).map(|ops| (m.name.clone(), ops)))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flat_map(|(name, ops)| ops.into_iter().map(move |op| (op.target, name.clone())))
        .fold(BTreeMap::new(), |mut acc, (target, name)| {
            acc.entry(target).or_insert_with(Vec::new).push(name);
            acc
        });
    let game = &config.game.dir;
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
