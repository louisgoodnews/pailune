//! Central table of staged mods with checkboxes and load-order controls.

use crate::mod_engine::model::ModEntry;
use crate::ui::theme::{self, MUTED};
use crate::ui::{Action, Confirm, PyluneApp};
use eframe::egui::{self, RichText};

/// Renders one row and returns the actions it triggered.
fn row(ui: &mut egui::Ui, index: usize, entry: &ModEntry, last: usize) -> Vec<Action> {
    let mut enabled = entry.enabled;
    let toggled = ui.checkbox(&mut enabled, "").changed();
    ui.label(RichText::new(&entry.name).strong());
    let (kind, color) = theme::kind_style(entry.kind);
    theme::badge(ui, kind, color);
    ui.label(RichText::new(entry.file_count.to_string()).color(MUTED));
    let up = ui
        .add_enabled(index > 0, egui::Button::new("▲").small())
        .clicked();
    let down = ui
        .add_enabled(index < last, egui::Button::new("▼").small())
        .clicked();
    let remove = ui
        .button("🗑")
        .on_hover_text("Delete from staging")
        .clicked();
    ui.end_row();
    [
        (toggled, Action::Toggle(index)),
        (
            up,
            Action::Move {
                from: index,
                to: index.saturating_sub(1),
            },
        ),
        (
            down,
            Action::Move {
                from: index,
                to: index + 1,
            },
        ),
        (remove, Action::Ask(Confirm::Remove(index))),
    ]
    .into_iter()
    .filter(|(clicked, _)| *clicked)
    .map(|(_, action)| action)
    .collect()
}

fn empty_hint(ui: &mut egui::Ui) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("No mods staged yet").size(18.0));
        ui.label(
            RichText::new("Import a folder, .zip, .asi, .dll or .json file to get started.")
                .color(MUTED),
        );
    });
}

/// Renders the mod table and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    ui.heading("Mods");
    ui.label(
        RichText::new("Load order runs top to bottom — later mods win file conflicts.")
            .small()
            .color(MUTED),
    );
    ui.separator();
    let mods = &app.registry.mods;
    if mods.is_empty() {
        empty_hint(ui);
        return Vec::new();
    }
    let last = mods.len() - 1;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("mod_grid")
                .striped(true)
                .num_columns(7)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ["On", "Name", "Type", "Files", "", "", ""]
                        .iter()
                        .for_each(|h| drop(ui.label(RichText::new(*h).small().color(MUTED))));
                    ui.end_row();
                    mods.iter()
                        .enumerate()
                        .flat_map(|(i, entry)| row(ui, i, entry, last))
                        .collect()
                })
                .inner
        })
        .inner
}
