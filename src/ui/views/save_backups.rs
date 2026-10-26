//! Save games section: back up, restore and delete copies of the game's
//! save files.

use super::clicked;
use crate::save_backup::SaveBackup;
use crate::ui::theme::muted;
use crate::ui::{Action, BackupAction, Confirm, PyluneApp};
use eframe::egui::{self, RichText};

/// One save backup: time, contents and its buttons.
fn row(ui: &mut egui::Ui, b: &SaveBackup) -> Vec<Action> {
    ui.label(super::log::timestamp(b.created_at));
    ui.label(
        RichText::new(format!(
            "{} file(s) · {:.1} MB · {}",
            b.files,
            b.size as f64 / 1e6,
            b.label
        ))
        .color(muted()),
    );
    let actions = clicked([
        (
            ui.small_button("Restore…").clicked(),
            Action::Ask(Confirm::RestoreSaves(b.path.clone())),
        ),
        (
            ui.small_button("🗑")
                .on_hover_text("Delete this backup")
                .clicked(),
            Action::Backup(BackupAction::Delete(b.path.clone())),
        ),
    ]);
    ui.end_row();
    actions
}

/// The player's save slots; clicking one opens it in the editor.
fn slots(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    if app.save_slots.is_empty() {
        return Vec::new();
    }
    ui.label(RichText::new("Your saves").strong());
    let mut actions = Vec::new();
    egui::Grid::new("save_slots")
        .striped(true)
        .num_columns(2)
        .show(ui, |ui| {
            app.save_slots.iter().for_each(|s| {
                let open = app.save_view.as_ref().is_some_and(|v| v.dir == s.dir);
                let title = super::save_view::slot_title(&s.name);
                let clicked = ui
                    .add(egui::Button::selectable(open, title))
                    .on_hover_text("Open this save on the right")
                    .clicked();
                if clicked {
                    actions.push(Action::Backup(BackupAction::InspectSave(s.dir.clone())));
                }
                ui.label(match (&s.error, &s.level) {
                    // The summary is a separate file; the save itself may still open.
                    (Some(err), _) => RichText::new(format!("no summary: {err}"))
                        .color(crate::ui::theme::warning()),
                    (None, level) => RichText::new(format!(
                        "level {} · {} · {:.1} MB",
                        level.as_deref().unwrap_or("?"),
                        s.saved_at.map(super::log::timestamp).unwrap_or_default(),
                        s.size as f64 / 1e6
                    ))
                    .color(muted()),
                });
                ui.end_row();
            });
        });
    ui.add_space(6.0);
    actions
}

/// Renders the section and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    let slot_actions = slots(ui, app);
    ui.separator();
    ui.label(RichText::new("Save backups").strong());
    ui.label(
        RichText::new(
            "Copies of the game's save files from the Proton prefix. Restoring backs up the \
             current saves first.",
        )
        .color(muted()),
    );
    let mut actions = slot_actions;
    actions.extend(
        ui.horizontal(|ui| {
            clicked([
                (
                    ui.button("💾 Back up saves now").clicked(),
                    Action::Backup(BackupAction::BackUpSaves),
                ),
                (
                    ui.button("📂 Open save folder").clicked(),
                    Action::Backup(BackupAction::OpenSaveFolder),
                ),
            ])
        })
        .inner,
    );
    match app.save_backups.is_empty() {
        true => drop(ui.label(RichText::new("No save backups yet.").color(muted()))),
        false => {
            egui::Grid::new("save_backups")
                .striped(true)
                .num_columns(4)
                .show(ui, |ui| {
                    app.save_backups
                        .iter()
                        .for_each(|b| actions.extend(row(ui, b)));
                });
        }
    }
    actions
}
