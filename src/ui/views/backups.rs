//! Backups section: create, restore and delete backups of Pylune's state.

use super::clicked;
use crate::app_backup::{BackupInfo, BackupKind};
use crate::ui::theme::MUTED;
use crate::ui::{Action, BackupAction, Confirm, PyluneApp};
use eframe::egui::{self, RichText};

fn row(ui: &mut egui::Ui, b: &BackupInfo) -> Vec<Action> {
    let kind = match b.kind {
        BackupKind::Full => "full",
        BackupKind::Settings => "settings + mod list",
    };
    ui.label(super::log::timestamp(b.created_at));
    ui.label(
        RichText::new(format!(
            "{kind} · {} mod(s) · {:.1} MB · {}",
            b.mods,
            b.size as f64 / 1e6,
            b.label
        ))
        .small()
        .color(MUTED),
    );
    let actions = clicked([
        (
            ui.small_button("Restore…").clicked(),
            Action::Ask(Confirm::RestoreBackup(b.path.clone())),
        ),
        (
            ui.small_button("🗑").clicked(),
            Action::Backup(BackupAction::Delete(b.path.clone())),
        ),
    ]);
    ui.end_row();
    actions
}

/// Renders the section and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    ui.heading("Pylune backups");
    ui.label(
        RichText::new(
            "Backups hold your settings, mod list (order, choices), profiles and collections; a full \
             backup also holds the mod files. An automatic backup is taken once a day.",
        )
        .small()
        .color(MUTED),
    );
    let mut actions = ui
        .horizontal(|ui| {
            clicked([
                (
                    ui.button("💾 Back up now").clicked(),
                    Action::Backup(BackupAction::Create { full: false }),
                ),
                (
                    ui.button("📦 Full backup")
                        .on_hover_text("Includes every mod file")
                        .clicked(),
                    Action::Backup(BackupAction::Create { full: true }),
                ),
                (
                    ui.button("Restore from file…").clicked(),
                    Action::Backup(BackupAction::RestoreFromFile),
                ),
                (
                    ui.button("📂")
                        .on_hover_text("Open the backup folder")
                        .clicked(),
                    Action::Backup(BackupAction::OpenFolder),
                ),
            ])
        })
        .inner;
    match app.backups.is_empty() {
        true => drop(ui.label(RichText::new("No backups yet.").small().color(MUTED))),
        false => {
            egui::Grid::new("backups")
                .striped(true)
                .num_columns(4)
                .show(ui, |ui| {
                    app.backups
                        .iter()
                        .take(15)
                        .for_each(|b| actions.extend(row(ui, b)));
                })
                .inner
        }
    }
    actions
}
