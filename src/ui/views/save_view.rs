//! Read-only viewer of a save slot (B-07): every object the save holds as
//! a collapsible tree, with a search box.

use crate::savefile::view::Node;
use crate::ui::theme::muted;
use crate::ui::{Action, BackupAction, PyluneApp};
use eframe::egui::{self, RichText};
use std::sync::Arc;

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

fn node(ui: &mut egui::Ui, n: &Node, hit: &Hits, path: &str, searching: bool) {
    match n.children.is_empty() {
        true => drop(ui.label(&n.label)),
        false => {
            egui::CollapsingHeader::new(&n.label)
                .id_salt(path)
                .open(searching.then_some(true))
                .show(ui, |ui| {
                    level(ui, &n.children, &hit.children, path, searching)
                });
        }
    }
}

fn level(ui: &mut egui::Ui, nodes: &[Node], hits: &[Hits], path: &str, searching: bool) {
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
        .for_each(|(i, n, h)| node(ui, n, h, &format!("{path}/{i}"), searching));
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

/// Renders the viewer while one is open.
pub fn show(ctx: &egui::Context, app: &mut PyluneApp) -> Vec<Action> {
    let Some((slot, tree, query)) = &mut app.save_view else {
        return Vec::new();
    };
    let mut open = true;
    egui::Window::new(format!("Save: {slot}"))
        .open(&mut open)
        .default_size([560.0, 520.0])
        .show(ctx, |ui| {
            ui.label(
                RichText::new(
                    "Read-only. Numbers like item or quest keys refer to the game's data tables.",
                )
                .color(muted()),
            );
            ui.add(
                egui::TextEdit::singleline(query)
                    .hint_text("🔍 Search, e.g. _level, ItemSaveData, 50001")
                    .desired_width(f32::INFINITY),
            );
            let needle = query.to_lowercase();
            let found = cached_hits(ui, slot, tree, &needle);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| level(ui, tree, &found, "save", !needle.is_empty()));
        });
    match open {
        true => Vec::new(),
        false => vec![Action::Backup(BackupAction::CloseSaveView)],
    }
}
