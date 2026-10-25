//! Central table of staged mods: selection, activation, order, status.

use super::clicked;
use crate::mod_engine::model::{ModEntry, MoveTo};
use crate::ui::theme::{self, ACCENT, DANGER, MUTED, WARNING};
use crate::ui::{Action, ModAction, PyluneApp};
use eframe::egui::{self, RichText};

/// Badges after the name: Nexus origin, update available, conflicts.
fn status_cell(ui: &mut egui::Ui, entry: &ModEntry, app: &PyluneApp) {
    ui.horizontal(|ui| {
        if entry.nexus.is_some() {
            theme::badge(ui, "NEXUS", MUTED);
        }
        if let Some(version) = app.nexus.updates.get(&entry.name) {
            theme::badge(ui, &format!("⬆ {version}"), WARNING);
        }
        if let Some(c) = app.mods.conflicts.get(&entry.name) {
            let color = if c.loses.is_empty() { ACCENT } else { DANGER };
            theme::badge(ui, &format!("⚔ {}/{}", c.wins.len(), c.loses.len()), color)
                .on_hover_text("Conflicts: files won / files lost");
        }
    });
}

/// Renders one row and returns the actions it triggered.
fn row(ui: &mut egui::Ui, index: usize, entry: &ModEntry, app: &PyluneApp) -> Vec<Action> {
    let last = app.registry.mods.len().saturating_sub(1);
    let name = entry.name.clone();
    ui.label(RichText::new(format!("{}", index + 1)).color(MUTED));
    let mut enabled = entry.enabled;
    let toggled = ui.checkbox(&mut enabled, "").changed();
    let selected = app.mods.selected.as_ref() == Some(&entry.name);
    let select = ui
        .selectable_label(selected, RichText::new(&entry.name).strong())
        .clicked();
    let (kind, color) = theme::kind_style(entry.kind);
    theme::badge(ui, kind, color);
    ui.label(RichText::new(entry.file_count.to_string()).color(MUTED));
    status_cell(ui, entry, app);
    let up = ui
        .add_enabled(index > 0, egui::Button::new("▲").small())
        .clicked();
    let down = ui
        .add_enabled(index < last, egui::Button::new("▼").small())
        .clicked();
    ui.end_row();
    clicked([
        (
            toggled,
            Action::Mod(ModAction::SetEnabled(name.clone(), enabled)),
        ),
        (select, Action::Mod(ModAction::Select(name.clone()))),
        (up, Action::Mod(ModAction::Move(name.clone(), MoveTo::Up))),
        (down, Action::Mod(ModAction::Move(name, MoveTo::Down))),
    ])
}

fn toolbar(ui: &mut egui::Ui) -> Vec<Action> {
    ui.horizontal(|ui| {
        clicked([
            (
                ui.button("📁 Import folder…").clicked(),
                Action::Mod(ModAction::ImportFolder),
            ),
            (
                ui.button("📦 Import archives / files…")
                    .on_hover_text(
                        ".zip .7z .rar .asi .dll .json — several at once run in parallel",
                    )
                    .clicked(),
                Action::Mod(ModAction::ImportFiles),
            ),
            (
                ui.button("⟳ Rescan").clicked(),
                Action::Mod(ModAction::Rescan),
            ),
            (
                ui.button("Enable all").clicked(),
                Action::Mod(ModAction::SetAll(true)),
            ),
            (
                ui.button("Disable all").clicked(),
                Action::Mod(ModAction::SetAll(false)),
            ),
        ])
    })
    .inner
}

/// Renders the mod table and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    let mut actions = toolbar(ui);
    ui.label(
        RichText::new("Load order runs top to bottom — later mods win file conflicts. Click a name for details.")
            .small()
            .color(MUTED),
    );
    ui.separator();
    let mods = &app.registry.mods;
    if mods.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No mods staged yet").size(18.0));
            ui.label(RichText::new("Import a folder or a .zip/.7z/.rar/.asi/.dll/.json file, or download from Nexus.").color(MUTED));
        });
        return actions;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("mod_grid")
                .striped(true)
                .num_columns(8)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ["#", "On", "Name", "Type", "Files", "Status", "", ""]
                        .iter()
                        .for_each(|h| drop(ui.label(RichText::new(*h).small().color(MUTED))));
                    ui.end_row();
                    let rows: Vec<Action> = mods
                        .iter()
                        .enumerate()
                        .flat_map(|(i, entry)| row(ui, i, entry, app))
                        .collect();
                    actions.extend(rows);
                });
        });
    actions
}
