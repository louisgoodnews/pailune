//! Modal dialog for packaging a staged mod for Nexus Mods upload.

use crate::mod_engine::package::PackageMetadata;
use crate::ui::theme::accent;
use crate::ui::{Action, ModAction};
use eframe::egui::{self, RichText};

/// Shows the modal if packaging is in progress; returns triggered actions.
pub fn show(ctx: &egui::Context, draft: &mut Option<PackageMetadata>) -> Vec<Action> {
    let Some(d) = draft else {
        return Vec::new();
    };
    let mut actions = Vec::new();
    let response = egui::Modal::new(egui::Id::unique("pailune_package_mod")).show(ctx, |ui| {
        ui.set_max_width(440.0);
        ui.heading("📦 Package mod for Nexus");
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Builds an installable .zip archive with modinfo.json and documentation, \
                 ready for upload to Nexus Mods or manual sharing.",
            )
            .small(),
        );
        ui.add_space(10.0);

        egui::Grid::new("package_mod_fields")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Mod name:");
                ui.label(RichText::new(&d.name).strong());
                ui.end_row();

                ui.label("Version:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut d.version)
                            .hint_text("1.1")
                            .desired_width(100.0),
                    );
                    ui.checkbox(&mut d.bump_version, "Update modinfo.json");
                });
                ui.end_row();

                ui.label("Author:");
                ui.add(
                    egui::TextEdit::singleline(&mut d.author)
                        .hint_text("Author name")
                        .desired_width(260.0),
                );
                ui.end_row();

                ui.label("Description:");
                ui.add(
                    egui::TextEdit::multiline(&mut d.description)
                        .hint_text("Mod description (written to README.md and modinfo)")
                        .desired_rows(3)
                        .desired_width(260.0),
                );
                ui.end_row();
            });

        ui.add_space(8.0);
        let sanitized = crate::mod_engine::install::sanitize_name(&d.name);
        let v_str = d.version.trim();
        let target_name = format!("{sanitized}-v{v_str}.zip");
        ui.label(
            RichText::new(format!("Target: exports/{target_name}"))
                .small()
                .monospace(),
        );
        ui.add_space(12.0);

        let can_package = !d.name.trim().is_empty() && !d.version.trim().is_empty();
        ui.horizontal(|ui| {
            let pkg_btn = egui::Button::new(
                RichText::new("📦 Build & Package .zip")
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(accent())
            .min_size(egui::vec2(0.0, 30.0));

            if ui.add_enabled(can_package, pkg_btn).clicked() {
                actions.push(Action::Mod(ModAction::PackageMod(d.clone())));
            }
            if ui.button("Cancel").clicked() {
                actions.push(Action::Mod(ModAction::ClosePackageMod));
            }
        });
    });

    if response.should_close() && actions.is_empty() {
        actions.push(Action::Mod(ModAction::ClosePackageMod));
    }

    actions
}
