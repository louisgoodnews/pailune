//! Central table of staged mods: selection, activation, order, status.

use super::clicked;
use crate::mod_engine::model::{ModEntry, MoveTo};
use crate::ui::theme::{self, accent, danger, muted, warning};
use crate::ui::{Action, ModAction, PailuneApp};
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder, TableRow};

/// Badges after the name: Nexus origin, update available, conflicts.
fn status_cell(ui: &mut egui::Ui, entry: &ModEntry, app: &PailuneApp) {
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

/// Renders one table row and returns the actions it triggered.
fn row(mut cells: TableRow, index: usize, entry: &ModEntry, app: &PailuneApp) -> Vec<Action> {
    let last = app.registry.mods.len().saturating_sub(1);
    let name = entry.name.clone();
    let selected = app.mods.selected.as_ref() == Some(&entry.name);
    cells.set_selected(selected);
    cells.col(|ui| drop(ui.label(RichText::new(format!("{}", index + 1)).color(muted()))));
    let mut enabled = entry.enabled;
    let mut toggled = false;
    cells.col(|ui| toggled = ui.checkbox(&mut enabled, "").changed());
    let mut select = false;
    cells.col(|ui| {
        select = ui
            .selectable_label(selected, RichText::new(&entry.name).strong())
            .clicked();
    });
    cells.col(|ui| {
        let (kind, color) = theme::kind_style(entry.kind);
        theme::badge(ui, kind, color);
    });
    cells.col(|ui| {
        let files = match entry.paz_files {
            0 => entry.file_count.to_string(),
            paz => format!("{} (+{paz} PAZ)", entry.file_count),
        };
        ui.label(RichText::new(files).color(muted()));
    });
    cells.col(|ui| status_cell(ui, entry, app));
    let (mut up, mut down) = (false, false);
    cells.col(|ui| {
        up = ui
            .add_enabled(index > 0, egui::Button::new("⏶").small())
            .clicked();
        down = ui
            .add_enabled(index < last, egui::Button::new("⏷").small())
            .clicked();
    });
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
pub fn show(ui: &mut egui::Ui, app: &mut PailuneApp) -> Vec<Action> {
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
    let app: &PailuneApp = app;
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
    actions.extend(trash(ui, app));
    // The name column takes the remaining width, so the table fills the
    // page; the other columns fit their content.
    let shown: Vec<(usize, &ModEntry)> = mods
        .iter()
        .enumerate()
        .filter(|(_, m)| needle.is_empty() || m.name.to_lowercase().contains(&needle))
        .collect();
    TableBuilder::new(ui)
        .id_salt("mod_table")
        .striped(true)
        .auto_shrink([false, false])
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto().at_least(24.0))
        .column(Column::auto())
        .column(Column::remainder().at_least(160.0).clip(true))
        .column(Column::auto())
        .column(Column::auto().at_least(40.0))
        .column(Column::auto().at_least(80.0))
        .column(Column::auto())
        .header(22.0, |mut header| {
            ["#", "On", "Name", "Type", "Files", "Status", "Order"]
                .iter()
                .for_each(|h| {
                    header.col(|ui| drop(ui.label(RichText::new(*h).small().color(muted()))));
                });
        })
        .body(|body| {
            body.rows(30.0, shown.len(), |cells| {
                let (index, entry) = shown[cells.index()];
                actions.extend(row(cells, index, entry, app));
            });
        });
    actions
}

/// "Recently deleted" mods with restore and permanent-delete buttons.
fn trash(ui: &mut egui::Ui, app: &PailuneApp) -> Vec<Action> {
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
