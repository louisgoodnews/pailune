//! Editing a mod's settings files (`*.ini`) in its details panel (D-12).
//! Unsaved text lives in egui's memory per file, so the panel stays a
//! read-only view of the app state.

use crate::mod_engine::model::ModEntry;
use crate::mod_engine::plugins::mod_settings;
use crate::ui::theme::muted;
use crate::ui::{Action, ModAction, PyluneApp};
use eframe::egui::{self, RichText};
use std::path::Path;

/// One file: a text editor with Save and Revert.
fn editor(ui: &mut egui::Ui, path: &Path, rel: &Path) -> Option<Action> {
    let id = egui::Id::unique(("pylune_ini", path));
    let saved = std::fs::read_to_string(path).unwrap_or_default();
    let mut text: String = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| saved.clone());
    let mut action = None;
    egui::CollapsingHeader::new(rel.display().to_string())
        .id_salt(id)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .code_editor()
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );
            let changed = text != saved;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(changed, egui::Button::new("💾 Save"))
                    .clicked()
                {
                    action = Some(Action::Mod(ModAction::SaveSettingsFile(
                        path.to_path_buf(),
                        text.clone(),
                    )));
                }
                if ui
                    .add_enabled(changed, egui::Button::new("Revert"))
                    .clicked()
                {
                    text = saved.clone();
                }
            });
        });
    ui.data_mut(|d| d.insert_temp(id, text));
    action
}

/// The settings section of a mod; empty when it ships no `.ini`.
pub fn show(ui: &mut egui::Ui, entry: &ModEntry, app: &PyluneApp) -> Vec<Action> {
    let root = app.paths.staging.join(&entry.name);
    let files = mod_settings(&root);
    if files.is_empty() {
        return Vec::new();
    }
    ui.separator();
    ui.label(RichText::new("Plugin settings").strong());
    ui.label(RichText::new("Changes apply the next time the game starts.").color(muted()));
    files
        .iter()
        .filter_map(|rel| editor(ui, &root.join(rel), rel))
        .collect()
}
