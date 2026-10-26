//! Diagnostics page: archive state, Proton hints, metadata backup health and
//! the locations of Pylune's files.

use crate::backup::MetaBackup;
use crate::launch::proton;
use crate::steam::locate::launch_options;
use crate::ui::theme::{self, danger, muted, success, warning};
use crate::ui::views::clicked;
use crate::ui::{Action, DeployAction, PyluneApp};
use eframe::egui::{self, RichText};

/// Label and colour of the metadata backup state.
fn meta_badge(meta: MetaBackup) -> (&'static str, egui::Color32) {
    match meta {
        MetaBackup::Current => ("META BACKUP OK", success()),
        MetaBackup::Outdated => ("BACKUP OUTDATED", warning()),
        MetaBackup::Missing => ("NO META BACKUP", danger()),
    }
}

/// Game version, registered pack groups and inactive archive folders.
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
                .color(danger()),
        );
    }
    if !status.unregistered.is_empty() {
        ui.label(
            RichText::new(format!(
                "Inactive archive folders (not loaded by the game, e.g. left by another mod manager): {}",
                status.unregistered.join(", ")
            ))
            .small()
            .color(warning()),
        );
    }
}

/// Archives, Proton prefix and the launch options ASI mods need.
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
            .color(muted()),
    );
    let proton = proton::setup(&app.info).map_or(
        "not found (start the game once from Steam)".to_string(),
        |s| s.proton_dir.display().to_string(),
    );
    ui.label(
        RichText::new(format!("Proton: {proton}"))
            .small()
            .color(muted()),
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
                    .color(warning()),
            ),
        ),
    }
}

/// Metadata backup state and set-aside archive folders.
fn backup_section(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    ui.heading("Game metadata backup");
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
                    .color(muted()),
            ),
        ),
        foreign => drop(
            ui.label(
                RichText::new(format!("Foreign overlays: {}", list(foreign)))
                    .small()
                    .color(warning()),
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

/// ASI plugins in the game folder and who installed them (D-12).
fn plugin_section(ui: &mut egui::Ui, app: &PyluneApp) {
    ui.heading("ASI plugins");
    if app.plugins.is_empty() {
        ui.label(RichText::new("No .asi plugins in bin64/, scripts/ or plugins/.").color(muted()));
        return;
    }
    egui::Grid::new("asi_plugins")
        .striped(true)
        .num_columns(3)
        .show(ui, |ui| {
            app.plugins.iter().for_each(|p| {
                ui.label(&p.name);
                match &p.owner {
                    Some(owner) => drop(ui.label(format!("from {owner}"))),
                    None => {
                        drop(ui.label(RichText::new("not installed by Pylune").color(warning())))
                    }
                }
                ui.label(
                    RichText::new(if p.ini.is_some() {
                        "has settings (.ini)"
                    } else {
                        ""
                    })
                    .color(muted()),
                );
                ui.end_row();
            });
        });
    ui.label(
        RichText::new("Plugins hook the game's code and often break after game updates; check their mod pages then.")
            .color(muted()),
    );
}

/// Renders the diagnostics page: archives, Proton, metadata backup, paths.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    proton_section(ui, app);
    ui.separator();
    plugin_section(ui, app);
    ui.separator();
    let actions = backup_section(ui, app);
    ui.separator();
    let paths = [
        ("Settings", &app.paths.config_file),
        ("Staging", &app.paths.staging),
        ("Database", &app.paths.database),
        ("Backups", &app.paths.backups),
        ("Logs", &app.paths.logs),
    ];
    paths.into_iter().for_each(|(what, path)| {
        ui.label(
            RichText::new(format!("{what}: {}", path.display()))
                .small()
                .color(muted()),
        );
    });
    actions
}
