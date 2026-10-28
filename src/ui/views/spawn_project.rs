//! Modal dialog for spawning a new mod project via the Pailune Modding SDK.

use crate::sdk::ProjectType;
use crate::ui::state::SpawnProjectDraft;
use crate::ui::theme::{accent, muted};
use crate::ui::{Action, ModAction};
use eframe::egui::{self, RichText};

/// Shows the modal if project spawning is active; returns triggered actions.
pub fn show(ctx: &egui::Context, draft: &mut Option<SpawnProjectDraft>) -> Vec<Action> {
    let Some(d) = draft else {
        return Vec::new();
    };
    let mut actions = Vec::new();
    let response = egui::Modal::new(egui::Id::unique("pailune_spawn_project")).show(ctx, |ui| {
        ui.set_max_width(480.0);
        ui.heading("🛠 Spawn new mod project (SDK)");
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Scaffolds a standalone mod project with ready-to-build templates, \
                 bundled libraries, entry points, and complete Markdown documentation.",
            )
            .small(),
        );
        ui.add_space(10.0);

        // Mod Type Selector
        ui.label(RichText::new("Mod Type:").strong());
        ui.horizontal(|ui| {
            ui.radio_value(&mut d.mod_type, ProjectType::Asi, "ASI Plugin (Native Rust / C++)");
            ui.radio_value(&mut d.mod_type, ProjectType::Json, "JSON Patch (Format 3)");
        });
        ui.label(
            RichText::new(match d.mod_type {
                ProjectType::Asi => {
                    "Native 64-bit Windows DLL loaded via proxy. Includes DllMain, threading, logger, and winapi bindings."
                }
                ProjectType::Json => {
                    "Declarative intent-based table modifications resolved against game tables with dry-run safety."
                }
            })
            .small()
            .color(muted()),
        );
        ui.add_space(8.0);

        egui::Grid::new("spawn_project_fields")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Project Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut d.name)
                        .hint_text("e.g. FastTravelPlus")
                        .desired_width(280.0),
                );
                ui.end_row();

                ui.label("Destination:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut d.location)
                            .hint_text("Folder path (e.g. /home/user/projects)")
                            .desired_width(200.0),
                    );
                    if ui.button("📁 Browse…").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            d.location = folder.display().to_string();
                        }
                    }
                });
                ui.end_row();

                ui.label("Author:");
                ui.add(
                    egui::TextEdit::singleline(&mut d.author)
                        .hint_text("Author name (optional)")
                        .desired_width(280.0),
                );
                ui.end_row();

                ui.label("Version:");
                ui.add(
                    egui::TextEdit::singleline(&mut d.version)
                        .hint_text("1.0")
                        .desired_width(280.0),
                );
                ui.end_row();

                ui.label("Description:");
                ui.add(
                    egui::TextEdit::multiline(&mut d.description)
                        .hint_text("Short summary of mod functionality (optional)")
                        .desired_rows(3)
                        .desired_width(280.0),
                );
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.checkbox(&mut d.stage_after, "Automatically stage in Pailune for instant testing");
        ui.add_space(12.0);

        let can_create = !d.name.trim().is_empty() && !d.location.trim().is_empty();
        ui.horizontal(|ui| {
            let create_btn = egui::Button::new(
                RichText::new("🚀 Create Project")
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(accent())
            .min_size(egui::vec2(0.0, 30.0));

            if ui.add_enabled(can_create, create_btn).clicked() {
                actions.push(Action::Mod(ModAction::SpawnProject(d.clone())));
            }
            if ui.button("Cancel").clicked() {
                actions.push(Action::Mod(ModAction::CloseSpawnProject));
            }
        });
    });

    if response.should_close() && actions.is_empty() {
        actions.push(Action::Mod(ModAction::CloseSpawnProject));
    }

    actions
}
