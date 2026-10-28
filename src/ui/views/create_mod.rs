//! Modal dialog for creating a new JSON mod in staging.

use crate::ui::state::CreateModDraft;
use crate::ui::theme::accent;
use crate::ui::{Action, ModAction};
use eframe::egui::{self, RichText};

/// Shows the modal if creation is in progress; returns triggered actions.
pub fn show(ctx: &egui::Context, draft: &mut Option<CreateModDraft>) -> Vec<Action> {
    let Some(d) = draft else {
        return Vec::new();
    };
    let mut actions = Vec::new();
    let response = egui::Modal::new(egui::Id::unique("pailune_create_mod")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        ui.heading("✨ Create new JSON mod");
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Creates an empty format-3 mod in staging, ready to edit with Pailune's JSON editor.",
            )
            .small(),
        );
        ui.add_space(8.0);
        egui::Grid::new("create_mod_fields")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut d.name)
                        .hint_text("e.g. Extra Carry Capacity")
                        .desired_width(260.0),
                );
                ui.end_row();

                ui.label("Author:");
                ui.add(
                    egui::TextEdit::singleline(&mut d.author)
                        .hint_text("Author name (optional)")
                        .desired_width(260.0),
                );
                ui.end_row();

                ui.label("Version:");
                ui.add(
                    egui::TextEdit::singleline(&mut d.version)
                        .hint_text("1.0")
                        .desired_width(260.0),
                );
                ui.end_row();

                ui.label("Description:");
                ui.add(
                    egui::TextEdit::multiline(&mut d.description)
                        .hint_text("What this mod changes (optional)")
                        .desired_rows(3)
                        .desired_width(260.0),
                );
                ui.end_row();
            });
        ui.add_space(12.0);
        let name_valid = !d.name.trim().is_empty();
        ui.horizontal(|ui| {
            let create_btn = egui::Button::new(
                RichText::new("✨ Create mod")
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(accent())
            .min_size(egui::vec2(0.0, 30.0));
            if ui.add_enabled(name_valid, create_btn).clicked() {
                actions.push(Action::Mod(ModAction::CreateMod {
                    name: d.name.clone(),
                    author: d.author.clone(),
                    version: d.version.clone(),
                    description: d.description.clone(),
                }));
            }
            if ui.button("Cancel").clicked() {
                actions.push(Action::Mod(ModAction::CloseCreateMod));
            }
            ui.separator();
            if ui
                .button("🛠 Spawn SDK Project…")
                .on_hover_text("Scaffold a standalone ASI or JSON project folder with templates & documentation")
                .clicked()
            {
                actions.push(Action::Mod(ModAction::CloseCreateMod));
                actions.push(Action::Mod(ModAction::SpawnProjectPrompt));
            }
        });
    });
    if response.should_close() && actions.is_empty() {
        actions.push(Action::Mod(ModAction::CloseCreateMod));
    }
    actions
}

