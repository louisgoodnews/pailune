//! Central table of staged mods: selection, activation, order, status.

use super::clicked;
use crate::mod_engine::model::{ModEntry, MoveTo};
use crate::ui::theme::{self, accent, danger, muted, warning};
use crate::ui::{Action, ModAction, PyluneApp};
use eframe::egui::{self, RichText};

/// Badges after the name: Nexus origin, update available, conflicts.
fn status_cell(ui: &mut egui::Ui, entry: &ModEntry, app: &PyluneApp) {
    ui.horizontal(|ui| {
        let collection = app.nexus.collections.iter().find(|c| {
            c.members
                .iter()
                .any(|m| m.mod_name.as_ref() == Some(&entry.name))
        });
        match collection {
            Some(c) => drop(theme::badge(ui, &format!("📚 {}", c.name), accent())),
            None if entry.nexus.is_some() => drop(theme::badge(ui, "NEXUS", muted())),
            None => {}
        }
        if let Some(version) = app.nexus.updates.get(&entry.name) {
            theme::badge(ui, &format!("⬆ {version}"), warning());
        }
        if let Some(c) = app.mods.conflicts.get(&entry.name) {
            let color = if c.loses.is_empty() {
                accent()
            } else {
                danger()
            };
            theme::badge(ui, &format!("⚔ {}/{}", c.wins.len(), c.loses.len()), color)
                .on_hover_text("Conflicts: files won / files lost");
        }
    });
}

/// Renders one row and returns the actions it triggered.
fn row(ui: &mut egui::Ui, index: usize, entry: &ModEntry, app: &PyluneApp) -> Vec<Action> {
    let last = app.registry.mods.len().saturating_sub(1);
    let name = entry.name.clone();
    ui.label(RichText::new(format!("{}", index + 1)).color(muted()));
    let mut enabled = entry.enabled;
    let toggled = ui.checkbox(&mut enabled, "").changed();
    let selected = app.mods.selected.as_ref() == Some(&entry.name);
    let select = ui
        .selectable_label(selected, RichText::new(&entry.name).strong())
        .clicked();
    let (kind, color) = theme::kind_style(entry.kind);
    theme::badge(ui, kind, color);
    let files = match entry.paz_files {
        0 => entry.file_count.to_string(),
        paz => format!("{} (+{paz} PAZ)", entry.file_count),
    };
    ui.label(RichText::new(files).color(muted()));
    status_cell(ui, entry, app);
    let up = ui
        .add_enabled(index > 0, egui::Button::new("⏶").small())
        .clicked();
    let down = ui
        .add_enabled(index < last, egui::Button::new("⏷").small())
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

/// Import, rescan and bulk enable buttons.
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
                        ".zip .7z .rar .asi .dll .json — several at once run in parallel (Ctrl+I)",
                    )
                    .clicked(),
                Action::Mod(ModAction::ImportFiles),
            ),
            (
                ui.button("⟳ Rescan")
                    .on_hover_text("Look for changes in the staging folder (F5)")
                    .clicked(),
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
pub fn show(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    let mut actions = toolbar(ui);
    ui.horizontal(|ui| {
        ui.label("🔍");
        ui.add(
            egui::TextEdit::singleline(&mut app.mods.filter)
                .id(crate::ui::shortcuts::search_id())
                .hint_text("Filter mods (Ctrl+F)")
                .desired_width(260.0),
        );
    });
    let app: &PyluneApp = app;
    let needle = app.mods.filter.trim().to_lowercase();
    ui.label(
        RichText::new("Load order runs top to bottom — later mods win file conflicts. Click a name for details.")
            .small()
            .color(muted()),
    );
    ui.separator();
    let mods = &app.registry.mods;
    if mods.is_empty() {
        ui.add_space(40.0);
        // An empty list teaches the next step and offers it (R-19).
        let empty = ui.vertical_centered(|ui| {
            ui.label(RichText::new("No mods yet").size(18.0));
            ui.label(RichText::new("Import a downloaded mod (.zip, .7z, .rar, folder, .asi, .dll, .json), drop it onto this window, or download one from Nexus Mods.").color(muted()));
            ui.label(RichText::new("Nothing in the game folder changes until you press Apply.").color(muted()));
            ui.add_space(6.0);
            clicked([
                (theme::filled_button(ui, "📦 Import archives / files…", accent()).clicked(), Action::Mod(ModAction::ImportFiles)),
                (ui.button("🌐 Find mods on Nexus").clicked(), Action::SwitchTab(crate::ui::Tab::Nexus)),
                (ui.button("❓ Getting started").clicked(), Action::Guide(crate::ui::guide::GuideAction::Open(crate::ui::guide::GuidePage::GettingStarted))),
            ])
        });
        actions.extend(empty.inner);
        actions.extend(trash(ui, app));
        return actions;
    }
    // Both directions, so narrow windows never clip columns.
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("mod_grid")
                .striped(true)
                .num_columns(8)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ["#", "On", "Name", "Type", "Files", "Status", "", ""]
                        .iter()
                        .for_each(|h| drop(ui.label(RichText::new(*h).small().color(muted()))));
                    ui.end_row();
                    let rows: Vec<Action> = mods
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| {
                            needle.is_empty() || m.name.to_lowercase().contains(&needle)
                        })
                        .flat_map(|(i, entry)| row(ui, i, entry, app))
                        .collect();
                    actions.extend(rows);
                });
        });
    actions.extend(trash(ui, app));
    actions
}

/// "Recently deleted" mods with restore and permanent-delete buttons.
fn trash(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    if app.mods.trash.is_empty() {
        return Vec::new();
    }
    egui::CollapsingHeader::new(format!("🗑 Recently deleted ({})", app.mods.trash.len()))
        .id_salt("trash")
        .show(ui, |ui| {
            app.mods
                .trash
                .iter()
                .flat_map(|item| {
                    ui.horizontal(|ui| {
                        ui.label(&item.name);
                        clicked([
                            (
                                ui.small_button("Restore").clicked(),
                                Action::Mod(ModAction::RestoreDeleted(item.id.clone())),
                            ),
                            (
                                ui.small_button("Delete forever").clicked(),
                                Action::Mod(ModAction::PurgeDeleted(item.id.clone())),
                            ),
                        ])
                    })
                    .inner
                })
                .collect()
        })
        .body_returned
        .unwrap_or_default()
}
