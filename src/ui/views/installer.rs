//! FOMOD installer options in the details panel: every visible step with
//! its groups, as radio buttons or checkboxes. Changes are stored at once
//! and take effect with the next Apply.

use crate::fomod::resolve::{default_selection, kind_of, selection_of, walk};
use crate::fomod::{Config, GroupKind, PluginKind, Selection};
use crate::mod_engine::model::{ModDetails, ModEntry};
use crate::ui::theme::{self, accent, muted};
use crate::ui::{Action, ModAction};
use eframe::egui::{self, RichText};

type Pick = (usize, usize, usize);

/// What a group asks of the user.
fn rule(kind: GroupKind) -> &'static str {
    match kind {
        GroupKind::ExactlyOne => "choose one",
        GroupKind::AtMostOne => "choose one or none",
        GroupKind::AtLeastOne => "choose at least one",
        GroupKind::All => "all included",
        GroupKind::Any => "choose any",
    }
}

/// The picks after clicking `pick`, respecting the group's rule.
fn toggled(picks: &[Pick], pick: Pick, kind: GroupKind) -> Vec<Pick> {
    let (s, g, _) = pick;
    let on = picks.contains(&pick);
    let in_group = |p: &&Pick| p.0 == s && p.1 == g;
    let others: Vec<Pick> = picks.iter().filter(|p| !in_group(p)).copied().collect();
    let group: Vec<Pick> = picks.iter().filter(in_group).copied().collect();
    let next_group: Vec<Pick> = match (kind, on) {
        (GroupKind::ExactlyOne, _) => vec![pick],
        (GroupKind::AtMostOne, true) => Vec::new(),
        (GroupKind::AtMostOne, false) => vec![pick],
        (GroupKind::AtLeastOne, true) if group.len() == 1 => group,
        (_, true) => group.into_iter().filter(|p| *p != pick).collect(),
        (_, false) => group.into_iter().chain([pick]).collect(),
    };
    others.into_iter().chain(next_group).collect()
}

/// One group's plugins; returns the new picks when the user clicked one.
fn group_ui(
    ui: &mut egui::Ui,
    config: &Config,
    picks: &[Pick],
    flags: &crate::fomod::resolve::Flags,
    (s, g): (usize, usize),
) -> Option<Vec<Pick>> {
    let group = &config.steps[s].groups[g];
    ui.label(RichText::new(format!("{} — {}", group.name, rule(group.kind))).color(muted()));
    let mut changed = None;
    group.plugins.iter().enumerate().for_each(|(p, plugin)| {
        let kind = kind_of(plugin, flags);
        let fixed = matches!(kind, PluginKind::Required | PluginKind::NotUsable)
            || group.kind == GroupKind::All;
        let on = picks.contains(&(s, g, p));
        let label = match kind {
            PluginKind::Recommended => format!("{} (recommended)", plugin.name),
            PluginKind::NotUsable => format!("{} (not usable)", plugin.name),
            _ => plugin.name.clone(),
        };
        let clicked = ui
            .add_enabled_ui(!fixed, |ui| match group.kind {
                GroupKind::ExactlyOne | GroupKind::AtMostOne => ui.radio(on, label).clicked(),
                _ => ui.checkbox(&mut on.clone(), label).clicked(),
            })
            .inner;
        if !plugin.description.is_empty() {
            ui.label(RichText::new(&plugin.description).small().color(muted()));
        }
        if clicked {
            changed = Some(toggled(picks, (s, g, p), group.kind));
        }
    });
    changed
}

/// Renders the installer options of a FOMOD mod; empty for other mods.
pub fn show(ui: &mut egui::Ui, entry: &ModEntry, details: &ModDetails) -> Vec<Action> {
    if let Some(err) = &details.installer_error {
        ui.separator();
        ui.horizontal(|ui| {
            theme::badge(ui, "INSTALLER", theme::danger());
            ui.label(RichText::new("Broken installer — this mod is not deployed").strong());
        });
        ui.label(RichText::new(err).color(muted()));
    }
    let Some(config) = &details.installer else {
        return Vec::new();
    };
    let stored = Selection::decode(entry.variant.as_deref());
    let state = walk(config, stored.as_ref());
    ui.separator();
    ui.horizontal(|ui| {
        theme::badge(ui, "INSTALLER", accent());
        ui.label(RichText::new(&config.name).strong());
    });
    if stored.is_none() {
        ui.label(RichText::new("Using the installer's recommended choices.").color(muted()));
    }
    let mut next = None;
    state.visible.iter().for_each(|&s| {
        ui.label(RichText::new(&config.steps[s].name).strong());
        (0..config.steps[s].groups.len()).for_each(|g| {
            if let Some(picks) = group_ui(ui, config, &state.picks, &state.flags, (s, g)) {
                next = Some(selection_of(config, &picks));
            }
        });
    });
    if stored.is_some() && ui.small_button("Back to the installer's choices").clicked() {
        next = Some(default_selection(config));
    }
    next.map(|selection| {
        Action::Mod(ModAction::SetVariant(
            entry.name.clone(),
            selection.encode(),
        ))
    })
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_rules_hold_when_clicking() {
        let picks = vec![(0, 0, 1), (0, 1, 0)];
        assert_eq!(
            toggled(&picks, (0, 0, 0), GroupKind::ExactlyOne),
            [(0, 1, 0), (0, 0, 0)]
        );
        assert_eq!(
            toggled(&picks, (0, 0, 1), GroupKind::AtMostOne),
            [(0, 1, 0)]
        );
        assert_eq!(
            toggled(&picks, (0, 0, 1), GroupKind::AtLeastOne),
            picks.iter().rev().copied().collect::<Vec<_>>()
        );
        assert_eq!(
            toggled(&picks, (0, 1, 1), GroupKind::Any),
            [(0, 0, 1), (0, 1, 0), (0, 1, 1)]
        );
    }
}
