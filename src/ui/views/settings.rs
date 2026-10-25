//! Right-hand panel: settings, Proton hints and backup health.

use crate::backup::MetaBackup;
use crate::steam::locate::launch_options;
use crate::ui::theme::{self, DANGER, MUTED, SUCCESS, WARNING};
use crate::ui::{Action, PyluneApp};
use eframe::egui::{self, RichText};

fn meta_badge(meta: MetaBackup) -> (&'static str, egui::Color32) {
    match meta {
        MetaBackup::Current => ("META BACKUP OK", SUCCESS),
        MetaBackup::Outdated => ("BACKUP OUTDATED", WARNING),
        MetaBackup::Missing => ("NO META BACKUP", DANGER),
    }
}

/// Text inputs bound to the settings draft; returns triggered actions.
fn settings_section(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    ui.heading("Settings");
    ui.label(RichText::new("Game directory").small().color(MUTED));
    let browse = ui
        .horizontal(|ui| {
            let browse = ui.button("…").clicked();
            ui.add(
                egui::TextEdit::singleline(&mut app.draft.game_dir).desired_width(f32::INFINITY),
            );
            browse
        })
        .inner;
    ui.label(RichText::new("Steam app id").small().color(MUTED));
    ui.text_edit_singleline(&mut app.draft.app_id);
    ui.label(
        RichText::new("JSON mod folder (relative to game)")
            .small()
            .color(MUTED),
    );
    ui.text_edit_singleline(&mut app.draft.json_subdir);
    let save = ui.button("Save settings").clicked();
    [
        (browse, Action::BrowseGameDir),
        (save, Action::SaveSettings),
    ]
    .into_iter()
    .filter(|(clicked, _)| *clicked)
    .map(|(_, a)| a)
    .collect()
}

fn proton_section(ui: &mut egui::Ui, app: &PyluneApp) {
    ui.heading("Proton");
    let prefix = app
        .info
        .prefix_dir
        .as_ref()
        .map_or("not found".to_string(), |p| p.display().to_string());
    ui.label(
        RichText::new(format!("Prefix: {prefix}"))
            .small()
            .color(MUTED),
    );
    match launch_options(app.loader) {
        Some(opts) => {
            ui.label(RichText::new("Steam launch options for ASI mods:").small());
            ui.code(opts);
        }
        None => drop(
            ui.label(
                RichText::new("Install an ASI loader (e.g. version.dll) to use ASI mods.")
                    .small()
                    .color(WARNING),
            ),
        ),
    }
}

fn backup_section(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    ui.heading("Backups");
    let (text, color) = meta_badge(app.backup.meta);
    theme::badge(ui, text, color);
    let foreign = &app.backup.foreign_overlays;
    let archived = &app.backup.archived_overlays;
    let name = |p: &std::path::PathBuf| p.file_name().map(|n| n.to_string_lossy().into_owned());
    let list = |paths: &Vec<std::path::PathBuf>| {
        paths.iter().filter_map(name).collect::<Vec<_>>().join(", ")
    };
    match foreign.is_empty() {
        true => drop(
            ui.label(
                RichText::new("No foreign overlay folders (> 0035)")
                    .small()
                    .color(MUTED),
            ),
        ),
        false => drop(
            ui.label(
                RichText::new(format!("Foreign overlays: {}", list(foreign)))
                    .small()
                    .color(WARNING),
            ),
        ),
    }
    let restore = !archived.is_empty()
        && ui
            .button(format!("Restore {} archived overlay(s)", archived.len()))
            .on_hover_text(list(archived))
            .clicked();
    let refresh = ui
        .button("Refresh metadata backup")
        .on_hover_text("Re-snapshot meta/ to .bak, e.g. after a game update")
        .clicked();
    [
        (restore, Action::RestoreOverlays),
        (refresh, Action::RefreshBackup),
    ]
    .into_iter()
    .filter(|(clicked, _)| *clicked)
    .map(|(_, a)| a)
    .collect()
}

/// Renders the side panel and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    egui::ScrollArea::vertical()
        .show(ui, |ui| {
            let mut actions = settings_section(ui, app);
            ui.separator();
            proton_section(ui, app);
            ui.separator();
            actions.extend(backup_section(ui, app));
            ui.separator();
            ui.label(
                RichText::new(format!("Staging: {}", app.paths.staging.display()))
                    .small()
                    .color(MUTED),
            );
            actions
        })
        .inner
}
