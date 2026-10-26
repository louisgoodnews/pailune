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

/// The player's save slots with a read-only inspector.
fn slots(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    if app.save_slots.is_empty() {
        return Vec::new();
    }
    ui.label(RichText::new("Your saves").strong());
    let mut actions = Vec::new();
    egui::Grid::new("save_slots")
        .striped(true)
        .num_columns(4)
        .show(ui, |ui| {
            app.save_slots.iter().for_each(|s| {
                let number: u32 = s.name.trim_start_matches("slot").parse().unwrap_or(0);
                let label = match number >= 100 {
                    true => format!("Autosave {}", number - 99),
                    false => format!("Slot {number}"),
                };
                ui.label(label);
                ui.label(match (&s.error, &s.level) {
                    (Some(err), _) => RichText::new(format!("unreadable: {err}"))
                        .color(crate::ui::theme::danger()),
                    (None, level) => RichText::new(format!(
                        "level {} · {} · {:.1} MB",
                        level.as_deref().unwrap_or("?"),
                        s.saved_at.map(super::log::timestamp).unwrap_or_default(),
                        s.size as f64 / 1e6
                    ))
                    .color(muted()),
                });
                if s.error.is_none()
                    && ui
                        .small_button("Inspect…")
                        .on_hover_text("Show everything the save contains (read-only)")
                        .clicked()
                {
                    actions.push(Action::Backup(BackupAction::InspectSave(s.dir.clone())));
                }
                ui.end_row();
            });
        });
    ui.add_space(6.0);
    actions
}

/// Renders the section and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    ui.heading("Save games");
    let slot_actions = slots(ui, app);
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
