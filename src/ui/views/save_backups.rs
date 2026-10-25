//! Save games section: back up, restore and delete copies of the game's
//! save files.

use super::clicked;
use crate::save_backup::SaveBackup;
use crate::ui::theme::muted;
use crate::ui::{Action, BackupAction, Confirm, PyluneApp};
use eframe::egui::{self, RichText};

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

/// Renders the section and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    ui.heading("Save games");
    ui.label(
        RichText::new(
            "Copies of the game's save files from the Proton prefix. Restoring backs up the \
             current saves first.",
        )
        .color(muted()),
    );
    let mut actions = ui
        .horizontal(|ui| {
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
        .inner;
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
