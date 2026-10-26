//! Read-only viewer of a save slot (B-07): every object the save holds as
//! a collapsible tree, with a search box.

use crate::savefile::view::Node;
use crate::ui::theme::muted;
use crate::ui::{Action, BackupAction, PyluneApp};
use eframe::egui::{self, RichText};

/// Children shown per level before "… more".
const SHOWN: usize = 300;

/// Whether a node or any descendant contains the search text.
fn matches(node: &Node, needle: &str) -> bool {
    needle.is_empty()
        || node.label.to_lowercase().contains(needle)
        || node.children.iter().any(|c| matches(c, needle))
}

fn node(ui: &mut egui::Ui, n: &Node, path: &str, needle: &str) {
    match n.children.is_empty() {
        true => drop(ui.label(&n.label)),
        false => {
            egui::CollapsingHeader::new(&n.label)
                .id_salt(path)
                .open((!needle.is_empty()).then_some(true))
                .show(ui, |ui| level(ui, &n.children, path, needle));
        }
    }
}

fn level(ui: &mut egui::Ui, nodes: &[Node], path: &str, needle: &str) {
    let visible: Vec<(usize, &Node)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches(n, needle))
        .collect();
    visible
        .iter()
        .take(SHOWN)
        .for_each(|(i, n)| node(ui, n, &format!("{path}/{i}"), needle));
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
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| level(ui, tree, "save", &needle));
        });
    match open {
        true => Vec::new(),
        false => vec![Action::Backup(BackupAction::CloseSaveView)],
    }
}
