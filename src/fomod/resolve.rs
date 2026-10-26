//! Turning an installer and the user's choices into files: flags, visible
//! steps, defaults, group rules, priorities and folder expansion.

use super::{Condition, Config, FileInstall, GroupKind, Plugin, PluginKind, Selection};
use crate::mod_engine::plan::resolve_case;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Condition flags set so far.
pub type Flags = HashMap<String, String>;

/// Whether `condition` holds. Game-file and version conditions count as met:
/// Pylune cannot know other managers' plugin states.
pub fn holds(condition: &Condition, flags: &Flags) -> bool {
    match condition {
        Condition::All(items) => items.iter().all(|c| holds(c, flags)),
        Condition::Any(items) => items.is_empty() || items.iter().any(|c| holds(c, flags)),
        Condition::Flag(name, value) => {
            flags.get(name).map(String::as_str).unwrap_or_default() == value
        }
        Condition::File(..) | Condition::Always => true,
    }
}

/// A plugin's type under the current flags.
pub fn kind_of(plugin: &Plugin, flags: &Flags) -> PluginKind {
    plugin
        .kind_rules
        .iter()
        .find(|(when, _)| holds(when, flags))
        .map_or(plugin.kind, |(_, kind)| *kind)
}

/// Default picks of one group: required ones, then recommended, then the
/// first usable plugin where the group needs one.
fn defaults(kind: GroupKind, plugins: &[Plugin], flags: &Flags) -> Vec<usize> {
    let of = |k: PluginKind| -> Vec<usize> {
        (0..plugins.len())
            .filter(|i| kind_of(&plugins[*i], flags) == k)
            .collect()
    };
    let usable = (0..plugins.len()).find(|i| kind_of(&plugins[*i], flags) != PluginKind::NotUsable);
    let (required, recommended) = (of(PluginKind::Required), of(PluginKind::Recommended));
    let chosen: Vec<usize> = match kind {
        GroupKind::All => (0..plugins.len()).collect(),
        GroupKind::ExactlyOne => required
            .first()
            .or(recommended.first())
            .copied()
            .or(usable)
            .into_iter()
            .collect(),
        GroupKind::AtMostOne => required
            .first()
            .or(recommended.first())
            .copied()
            .into_iter()
            .collect(),
        GroupKind::AtLeastOne => match [required.clone(), recommended.clone()].concat() {
            picks if picks.is_empty() => usable.into_iter().collect(),
            picks => picks,
        },
        GroupKind::Any => [required, recommended].concat(),
    };
    chosen
}

/// Result of walking the installer with a selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Walk {
    /// `(step, group, plugin)` indices that count.
    pub picks: Vec<(usize, usize, usize)>,
    /// Flags after the last step.
    pub flags: Flags,
    /// Indices of the steps shown to the user.
    pub visible: Vec<usize>,
}

/// The plugins that count, step by step. Groups the selection says nothing
/// about get defaults; required plugins are always in, unusable ones never.
pub fn walk(config: &Config, selection: Option<&Selection>) -> Walk {
    let mut flags = Flags::new();
    let mut out = Vec::new();
    let mut visible = Vec::new();
    for (si, step) in config.steps.iter().enumerate() {
        if !step.visible.as_ref().is_none_or(|c| holds(c, &flags)) {
            continue;
        }
        visible.push(si);
        for (gi, group) in step.groups.iter().enumerate() {
            let chosen_by_user: Vec<usize> = (0..group.plugins.len())
                .filter(|pi| {
                    selection
                        .is_some_and(|s| s.has(&step.name, &group.name, &group.plugins[*pi].name))
                })
                .collect();
            let mut chosen = match (selection, chosen_by_user.is_empty(), group.kind) {
                (Some(_), false, _) | (Some(_), true, GroupKind::AtMostOne | GroupKind::Any) => {
                    chosen_by_user
                }
                _ => defaults(group.kind, &group.plugins, &flags),
            };
            let required: Vec<usize> = (0..group.plugins.len())
                .filter(|pi| kind_of(&group.plugins[*pi], &flags) == PluginKind::Required)
                .filter(|pi| !chosen.contains(pi))
                .collect();
            chosen.extend(required);
            chosen.retain(|pi| kind_of(&group.plugins[*pi], &flags) != PluginKind::NotUsable);
            chosen.sort_unstable();
            for pi in chosen {
                group.plugins[pi].flags.iter().for_each(|(k, v)| {
                    flags.insert(k.clone(), v.clone());
                });
                out.push((si, gi, pi));
            }
        }
    }
    Walk {
        picks: out,
        flags,
        visible,
    }
}

/// The choices an untouched installer makes (used until the user picks).
pub fn default_selection(config: &Config) -> Selection {
    selection_of(config, &walk(config, None).picks)
}

/// The names of the given picks, as a storable selection.
pub fn selection_of(config: &Config, picks: &[(usize, usize, usize)]) -> Selection {
    Selection {
        picks: picks
            .iter()
            .map(|(s, g, p)| {
                let step = &config.steps[*s];
                let group = &step.groups[*g];
                (
                    step.name.clone(),
                    group.name.clone(),
                    group.plugins[*p].name.clone(),
                )
            })
            .collect(),
    }
}

/// A FOMOD path (`\\` or `/`, any case) as a plain relative path; `None`
/// for anything that could leave its folder (`..`, drive or root prefixes).
fn rel_path(text: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = text
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    let unsafe_part = |p: &&str| *p == ".." || p.contains(':');
    match parts.iter().any(unsafe_part) {
        true => None,
        false => Some(parts.into_iter().collect()),
    }
}

/// Whether `path` lies inside `root` once symlinks are resolved.
fn inside(root: &Path, path: &Path) -> bool {
    match (root.canonicalize(), path.canonicalize()) {
        (Ok(root), Ok(path)) => path.starts_with(root),
        _ => false,
    }
}

/// Files of one install: `(source, destination)`. Installs whose paths
/// would leave the mod or the game folder are dropped.
fn expand(mod_root: &Path, install: &FileInstall) -> Vec<(PathBuf, PathBuf)> {
    let (Some(source_rel), Some(dest)) =
        (rel_path(&install.source), rel_path(&install.destination))
    else {
        tracing::warn!(source = %install.source, destination = %install.destination, "FOMOD path leaves its folder; ignored");
        return Vec::new();
    };
    let source = resolve_case(mod_root, &source_rel);
    if !inside(mod_root, &source) {
        return Vec::new();
    }
    match install.folder {
        false => {
            let dest = match dest.as_os_str().is_empty() {
                true => source_rel,
                false => dest,
            };
            source
                .is_file()
                .then_some((source, dest))
                .into_iter()
                .collect()
        }
        true => WalkDir::new(&source)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let inner = e
                    .path()
                    .strip_prefix(&source)
                    .unwrap_or(e.path())
                    .to_path_buf();
                (e.into_path(), dest.join(inner))
            })
            .collect(),
    }
}

/// Every file to deploy, `(staged source, path in the game layout)`; when
/// two installs write the same path, the higher priority (then the later
/// one) wins.
pub fn resolve(
    mod_root: &Path,
    config: &Config,
    selection: Option<&Selection>,
) -> Vec<(PathBuf, PathBuf)> {
    let Walk {
        picks: chosen,
        flags,
        ..
    } = walk(config, selection);
    let plugin_files = chosen
        .iter()
        .flat_map(|(s, g, p)| config.steps[*s].groups[*g].plugins[*p].files.iter());
    let conditional = config
        .conditional
        .iter()
        .filter(|c| holds(&c.condition, &flags))
        .flat_map(|c| c.files.iter());
    let mut installs: Vec<&FileInstall> = config
        .required
        .iter()
        .chain(plugin_files)
        .chain(conditional)
        .collect();
    installs.sort_by_key(|i| i.priority);
    let mut by_dest: BTreeMap<String, (PathBuf, PathBuf)> = BTreeMap::new();
    installs
        .iter()
        .flat_map(|i| expand(mod_root, i))
        .for_each(|(src, dest)| {
            by_dest.insert(dest.to_string_lossy().to_lowercase(), (src, dest));
        });
    by_dest.into_values().collect()
}
