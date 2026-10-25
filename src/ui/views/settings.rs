//! Settings page: config.toml values, Proton hints and backup health.

use super::clicked;
use crate::backup::MetaBackup;
use crate::launch::proton;
use crate::steam::locate::launch_options;
use crate::ui::theme::{self, DANGER, MUTED, SUCCESS, WARNING};
use crate::ui::{Action, DeployAction, PyluneApp};
use eframe::egui::{self, RichText};

fn meta_badge(meta: MetaBackup) -> (&'static str, egui::Color32) {
    match meta {
        MetaBackup::Current => ("META BACKUP OK", SUCCESS),
        MetaBackup::Outdated => ("BACKUP OUTDATED", WARNING),
        MetaBackup::Missing => ("NO META BACKUP", DANGER),
    }
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(RichText::new(label).color(MUTED));
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(f32::INFINITY),
    );
    ui.end_row();
}

fn settings_section(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    ui.heading("Settings");
    ui.label(
        RichText::new(format!("Stored in {}", app.paths.config_file.display()))
            .small()
            .color(MUTED),
    );
    let draft = &mut app.draft;
    let browse = egui::Grid::new("settings_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Game directory").color(MUTED));
            let browse = ui
                .horizontal(|ui| {
                    let browse = ui.button("…").clicked();
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.game_dir)
                            .desired_width(f32::INFINITY),
                    );
                    browse
                })
                .inner;
            ui.end_row();
            field(ui, "Steam app id", &mut draft.app_id, "3321460");
            field(
                ui,
                "Nexus game domain",
                &mut draft.nexus_domain,
                "crimsondesert",
            );
            field(
                ui,
                "Default launch options",
                &mut draft.default_options,
                "used when the active profile has none",
            );
            field(
                ui,
                "Parallel jobs",
                &mut draft.max_parallel,
                "1–16 (restart required)",
            );
            field(
                ui,
                "Log level",
                &mut draft.log_level,
                "info, debug, pylune=debug … (restart required)",
            );
            ui.label("");
            ui.checkbox(
                &mut draft.apply_before_launch,
                "Apply pending changes before Play",
            );
            ui.end_row();
            browse
        })
        .inner;
    let save = ui.button("💾 Save settings").clicked();
    clicked([
        (browse, Action::BrowseGameDir),
        (save, Action::SaveSettings),
    ])
}

fn archive_section(ui: &mut egui::Ui, app: &PyluneApp) {
    ui.heading("Game archives");
    let status = &app.archives;
    let version = status.version.as_deref().unwrap_or("unknown");
    ui.label(
        RichText::new(format!(
            "Game version {version} · {} pack groups registered",
            status.registered
        ))
        .small(),
    );
    if let Some(err) = &status.error {
        ui.label(
            RichText::new(format!("Cannot read meta/0.papgt: {err}"))
                .small()
                .color(DANGER),
        );
    }
    if !status.unregistered.is_empty() {
        ui.label(
            RichText::new(format!(
                "Inactive archive folders (not loaded by the game, e.g. left by another mod manager): {}",
                status.unregistered.join(", ")
            ))
            .small()
            .color(WARNING),
        );
    }
}

fn proton_section(ui: &mut egui::Ui, app: &PyluneApp) {
    archive_section(ui, app);
    ui.separator();
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
    let proton = proton::setup(&app.info).map_or(
        "not found (start the game once from Steam)".to_string(),
        |s| s.proton_dir.display().to_string(),
    );
    ui.label(
        RichText::new(format!("Proton: {proton}"))
            .small()
            .color(MUTED),
    );
    match launch_options(app.loader) {
        Some(opts) => {
            ui.label(
                RichText::new("Steam launch options (or a profile's options) for ASI mods:")
                    .small(),
            );
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
    let name = |p: &std::path::PathBuf| p.file_name().map(|n| n.to_string_lossy().into_owned());
    let list =
        |paths: &[std::path::PathBuf]| paths.iter().filter_map(name).collect::<Vec<_>>().join(", ");
    match app.backup.foreign_overlays.as_slice() {
        [] => drop(
            ui.label(
                RichText::new("No foreign overlay folders (> 0035)")
                    .small()
                    .color(MUTED),
            ),
        ),
        foreign => drop(
            ui.label(
                RichText::new(format!("Foreign overlays: {}", list(foreign)))
                    .small()
                    .color(WARNING),
            ),
        ),
    }
    let archived = &app.backup.archived_overlays;
    let restore = !archived.is_empty()
        && ui
            .button(format!("Restore {} archived overlay(s)", archived.len()))
            .on_hover_text(list(archived))
            .clicked();
    let refresh = ui
        .button("Refresh metadata backup")
        .on_hover_text("Re-snapshot meta/ to .bak, e.g. after a game update")
        .clicked();
    clicked([
        (restore, Action::Deploy(DeployAction::RestoreOverlays)),
        (refresh, Action::Deploy(DeployAction::RefreshBackup)),
    ])
}

/// Renders the settings page and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &mut PyluneApp) -> Vec<Action> {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut actions = settings_section(ui, app);
            ui.separator();
            proton_section(ui, app);
            ui.separator();
            actions.extend(super::backups::show(ui, app));
            ui.separator();
            actions.extend(backup_section(ui, app));
            ui.separator();
            ui.label(
                RichText::new(format!("Staging: {}", app.paths.staging.display()))
                    .small()
                    .color(MUTED),
            );
            ui.label(
                RichText::new(format!("Database: {}", app.paths.database.display()))
                    .small()
                    .color(MUTED),
            );
            actions
        })
        .inner
}
