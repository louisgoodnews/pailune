//! Right-hand panel for the selected mod: actions, info, conflicts, files.

use crate::mod_engine::conflicts::{Contest, ModConflicts};
use crate::mod_engine::model::{ModDetails, ModEntry};
use crate::ui::theme::{self, accent, muted, success, warning};
use crate::ui::{Action, PyluneApp};
use eframe::egui::{self, RichText};

/// State, file counts, size and source of the mod.
fn info_grid(ui: &mut egui::Ui, entry: &ModEntry, details: &ModDetails) {
    egui::Grid::new("mod_info")
        .num_columns(2)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            let row = |ui: &mut egui::Ui, key: &str, value: String| {
                ui.label(RichText::new(key).color(muted()));
                ui.label(value);
                ui.end_row();
            };
            row(
                ui,
                "State",
                if entry.enabled {
                    "enabled".into()
                } else {
                    "disabled".into()
                },
            );
            row(
                ui,
                "Files",
                format!(
                    "{} linkable / {} total",
                    entry.file_count,
                    details.files.len()
                ),
            );
            if entry.paz_files > 0 {
                ui.label(RichText::new("Needs PAZ").color(warning()));
                ui.label(
                    RichText::new(format!(
                        "{} file(s) are JSON patches or RAW/loose game files. The game only reads \
                     these from its PAZ archives, so they are not deployed until Pylune can \
                     pack archives (roadmap phase 3).",
                        entry.paz_files
                    ))
                    .color(warning()),
                );
                ui.end_row();
            }
            row(
                ui,
                "Size",
                format!("{:.2} MiB", details.total_bytes as f64 / 1_048_576.0),
            );
            row(
                ui,
                "Source",
                if entry.source.is_empty() {
                    "unknown".into()
                } else {
                    entry.source.clone()
                },
            );
            if let Some(n) = &entry.nexus {
                row(
                    ui,
                    "Nexus",
                    format!("mod {} · file {} · v{}", n.mod_id, n.file_id, n.version),
                );
            }
        });
}

/// Files won or lost against other mods.
fn contest_list(ui: &mut egui::Ui, title: &str, contests: &[Contest], verb: &str) {
    if contests.is_empty() {
        return;
    }
    egui::CollapsingHeader::new(format!("{title} ({})", contests.len()))
        .id_salt(title)
        .show(ui, |ui| {
            contests.iter().for_each(|c| {
                ui.label(
                    RichText::new(format!("{} — {verb} {}", c.file.display(), c.other))
                        .small()
                        .monospace(),
                );
            });
        });
}

/// Conflict summary of the selected mod.
fn conflicts(ui: &mut egui::Ui, report: Option<&ModConflicts>, enabled: bool, analysing: bool) {
    ui.label(RichText::new("File conflicts").strong());
    match (report, enabled) {
        (None, true) if analysing => {
            drop(ui.label(RichText::new("Analysing…").small().color(muted())))
        }
        (_, false) => drop(
            ui.label(
                RichText::new("Enable the mod to see its conflicts.")
                    .small()
                    .color(muted()),
            ),
        ),
        (None, true) => drop(
            ui.label(
                RichText::new("No conflicts with other active mods.")
                    .small()
                    .color(success()),
            ),
        ),
        (Some(c), true) => {
            contest_list(ui, "Overrides", &c.wins, "overrides");
            contest_list(ui, "Overridden", &c.loses, "overridden by");
        }
    }
}

/// Renders the details panel and returns triggered actions.
pub fn show(ui: &mut egui::Ui, app: &PyluneApp) -> Vec<Action> {
    let selected = app.mods.selected.as_ref();
    let entry = selected.and_then(|name| app.registry.mods.iter().find(|m| &m.name == name));
    let details = app
        .mods
        .details
        .as_ref()
        .filter(|(name, _)| Some(name) == selected)
        .map(|(_, d)| d);
    let (Some(entry), Some(details)) = (entry, details) else {
        ui.label(RichText::new("Select a mod.").color(muted()));
        return Vec::new();
    };
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(
                RichText::new(&entry.name)
                    .size(18.0)
                    .strong()
                    .color(accent()),
            );
            let (kind, color) = theme::kind_style(entry.kind);
            theme::badge(ui, kind, color);
            ui.add_space(4.0);
            let mut actions = super::mod_actions::show(ui, entry, app);
            ui.separator();
            info_grid(ui, entry, details);
            actions.extend(super::variant::show(ui, entry, details));
            actions.extend(super::installer::show(ui, entry, details));
            ui.separator();
            conflicts(
                ui,
                app.mods.conflicts.get(&entry.name),
                entry.enabled,
                app.analysis_active(),
            );
            ui.separator();
            ui.label(RichText::new("Readme & documentation").strong());
            if details.readmes.is_empty() {
                ui.label(
                    RichText::new("No readme found in this mod.")
                        .small()
                        .color(muted()),
                );
            }
            actions.extend(super::mod_files::show(ui, details));
            actions
        })
        .inner
}
