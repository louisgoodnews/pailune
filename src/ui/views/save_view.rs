//! Viewer and editor of a save slot (B-07): every object the save holds as
//! a collapsible tree with a search box; fixed-size values are text fields,
//! written together with "Save changes" after a confirmation.

use crate::savefile::edit::parse_value;
use crate::savefile::view::{Editable, Node, item_count, scalar};
use crate::ui::state::SaveView;
use crate::ui::theme::{accent, danger, muted};
use crate::ui::{Action, BackupAction, PyluneApp};
use eframe::egui::{self, RichText};
use std::collections::BTreeMap;
use std::sync::Arc;

/// What every level of the tree needs: search state and the typed values.
struct Walk<'a> {
    searching: bool,
    edits: &'a mut BTreeMap<usize, String>,
}

/// One editable value: its field name and a text field. Accent while
/// changed (with the old value), red while not a valid value of its type.
fn value_field(ui: &mut egui::Ui, value: &Editable, edits: &mut BTreeMap<usize, String>) {
    let current = scalar(&value.type_name, &value.bytes);
    let mut text = edits
        .get(&value.at)
        .cloned()
        .unwrap_or_else(|| current.clone());
    ui.horizontal(|ui| {
        ui.label(format!("{} =", value.field));
        let valid = parse_value(&value.type_name, value.bytes.len(), &text).is_ok();
        let changed = text != current;
        let mut field = egui::TextEdit::singleline(&mut text).desired_width(140.0);
        field = match (valid, changed) {
            (false, _) => field.text_color(danger()),
            (true, true) => field.text_color(accent()),
            (true, false) => field,
        };
        let response = ui.add(field).on_hover_text(&value.type_name);
        if changed {
            ui.label(
                RichText::new(format!("was {current}"))
                    .small()
                    .color(muted()),
            );
        }
        if response.changed() {
            match text == current {
                true => edits.remove(&value.at),
                false => edits.insert(value.at, text.clone()),
            };
        }
    });
}

/// Children shown per level before "… more".
const SHOWN: usize = 300;

/// Which nodes match the search: a node is shown when it or a descendant
/// contains the text. Computed once per query, not every frame.
#[derive(Debug, Default)]
struct Hits {
    shown: bool,
    children: Vec<Hits>,
}

fn hits(node: &Node, needle: &str) -> Hits {
    let children: Vec<Hits> = node.children.iter().map(|c| hits(c, needle)).collect();
    Hits {
        shown: needle.is_empty()
            || node.label.to_lowercase().contains(needle)
            || children.iter().any(|c| c.shown),
        children,
    }
}

fn node(ui: &mut egui::Ui, n: &Node, hit: &Hits, path: &str, walk: &mut Walk) {
    match (&n.edit, n.children.is_empty()) {
        (Some(value), _) => value_field(ui, value, walk.edits),
        (None, true) => drop(ui.label(&n.label)),
        (None, false) => {
            egui::CollapsingHeader::new(&n.label)
                .id_salt(path)
                .open(walk.searching.then_some(true))
                .show(ui, |ui| level(ui, &n.children, &hit.children, path, walk));
        }
    }
}

fn level(ui: &mut egui::Ui, nodes: &[Node], hits: &[Hits], path: &str, walk: &mut Walk) {
    let visible: Vec<(usize, &Node, &Hits)> = nodes
        .iter()
        .zip(hits)
        .enumerate()
        .filter(|(_, (_, h))| h.shown)
        .map(|(i, (n, h))| (i, n, h))
        .collect();
    visible
        .iter()
        .take(SHOWN)
        .for_each(|(i, n, h)| node(ui, n, h, &format!("{path}/{i}"), walk));
    if visible.len() > SHOWN {
        ui.label(
            RichText::new(format!(
                "… {} more (narrow the search)",
                visible.len() - SHOWN
            ))
            .color(muted()),
        );
    }
}

/// Tree address and search text a cached result belongs to.
type CacheKey = (usize, String);

/// The search result for `needle`, cached in egui's memory by slot, tree
/// (a reopened slot is a new tree) and text.
fn cached_hits(ui: &egui::Ui, slot: &str, tree: &[Node], needle: &str) -> Arc<Vec<Hits>> {
    let id = egui::Id::unique(("pylune_save_hits", slot));
    let key = (tree.as_ptr() as usize, needle.to_string());
    let cached: Option<(CacheKey, Arc<Vec<Hits>>)> = ui.data(|d| d.get_temp(id));
    match cached {
        Some((query, hits)) if query == key => hits,
        _ => {
            let fresh = Arc::new(tree.iter().map(|n| hits(n, needle)).collect::<Vec<_>>());
            ui.data_mut(|d| d.insert_temp(id, (key, fresh.clone())));
            fresh
        }
    }
}

/// Coins shortcut, pending changes and the Save / Discard buttons.
fn edit_bar(ui: &mut egui::Ui, view: &mut SaveView) -> Vec<Action> {
    if let Some(coins) = item_count(&view.tree, 1).cloned() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Copper coins in the pouch:").strong());
            value_field(ui, &coins, &mut view.edits);
        });
    }
    let pending = view.edits.len();
    let changes = match pending {
        0 => Ok(Vec::new()),
        _ => crate::ui::actions::save_changes(view),
    };
    let valid = changes.is_ok();
    // What will be written, so nothing has to be found in the tree again.
    match &changes {
        Ok(list) => list.iter().for_each(|c| {
            ui.label(
                RichText::new(format!("• {}", c.label))
                    .small()
                    .color(accent()),
            );
        }),
        Err(err) => drop(ui.label(RichText::new(err.to_string()).small().color(danger()))),
    }
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        let label = format!("💾 Save changes ({pending})…");
        if ui
            .add_enabled(pending > 0 && valid, egui::Button::new(label))
            .on_disabled_hover_text("Change a value first; red values are not valid")
            .clicked()
        {
            actions.push(Action::Backup(BackupAction::SaveEdits));
        }
        if ui
            .add_enabled(pending > 0, egui::Button::new("Discard"))
            .clicked()
        {
            actions.push(Action::Backup(BackupAction::DiscardSaveEdits));
        }
    });
    actions
}

/// Renders the viewer while one is open.
pub fn show(ctx: &egui::Context, app: &mut PyluneApp) -> Vec<Action> {
    let Some(view) = &mut app.save_view else {
        return Vec::new();
    };
    let mut open = true;
    let mut actions = Vec::new();
    egui::Window::new(format!("Save: {}", view.slot))
        .open(&mut open)
        .default_size([600.0, 560.0])
        .show(ctx, |ui| {
            ui.label(
                RichText::new(
                    "Change values, then Save changes — all saves are backed up first. Numbers \
                     like item or quest keys refer to the game's data tables.",
                )
                .color(muted()),
            );
            actions.extend(edit_bar(ui, view));
            ui.add(
                egui::TextEdit::singleline(&mut view.query)
                    .hint_text("🔍 Search, e.g. _level, ItemSaveData, 50001")
                    .desired_width(f32::INFINITY),
            );
            let needle = view.query.to_lowercase();
            let found = cached_hits(ui, &view.slot, &view.tree, &needle);
            let mut walk = Walk {
                searching: !needle.is_empty(),
                edits: &mut view.edits,
            };
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| level(ui, &view.tree, &found, "save", &mut walk));
        });
    if !open {
        actions.push(Action::Backup(BackupAction::CloseSaveView));
    }
    actions
}
