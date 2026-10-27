//! Editing a mod's settings files (`*.ini`) in its details panel (D-12).
//! Unsaved text lives in egui's memory per file, so the panel stays a
//! read-only view of the app state.

use crate::mod_engine::model::{ModDetails, ModEntry};
use crate::ui::theme::{danger, muted};
use crate::ui::{Action, ModAction, PailuneApp};
use eframe::egui::{self, RichText};
use std::path::{Path, PathBuf};

/// An edit in progress: the file text it started from, and the text now.
#[derive(Clone)]
struct Draft {
    base: String,
    text: String,
}

/// One file: a text editor with Save and Revert. The file is read only
/// while its section is open; a draft is kept only while it differs.
fn editor(ui: &mut egui::Ui, path: &Path, rel: &Path) -> Option<Action> {
    let id = egui::Id::unique(("pailune_ini", path));
    let mut action = None;
    egui::CollapsingHeader::new(rel.display().to_string())
        .id_salt(id)
        .show(ui, |ui| {
            let saved = std::fs::read_to_string(path).unwrap_or_default();
            let mut draft: Draft = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| Draft {
                base: saved.clone(),
                text: saved.clone(),
            });
            ui.add(
                egui::TextEdit::multiline(&mut draft.text)
                    .code_editor()
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );
            if draft.base != saved {
                ui.label(
                    RichText::new("The file changed since you started editing — Revert loads it.")
                        .color(danger()),
                );
            }
            let changed = draft.text != saved;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(changed, egui::Button::new("💾 Save"))
                    .clicked()
                {
                    action = Some(Action::Mod(ModAction::SaveModFile(
                        path.to_path_buf(),
                        draft.text.clone(),
                    )));
                    draft.base = draft.text.clone();
                }
                if ui
                    .add_enabled(changed, egui::Button::new("Revert"))
                    .clicked()
                {
                    draft.text = saved.clone();
                    draft.base = saved.clone();
                }
            });
            match draft.text == draft.base {
                true => ui.data_mut(|d| d.remove::<Draft>(id)),
                false => ui.data_mut(|d| {
                    d.insert_temp(id, draft);
                }),
            }
        });
    action
}

/// The settings files among the mod's (already listed) files.
fn settings_files(details: &ModDetails) -> Vec<PathBuf> {
    details
        .files
        .iter()
        .map(|f| f.rel.clone())
        .filter(|rel| {
            rel.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("ini"))
        })
        .collect()
}

/// The settings section of a mod; empty when it ships no `.ini`.
pub fn show(
    ui: &mut egui::Ui,
    entry: &ModEntry,
    details: &ModDetails,
    app: &PailuneApp,
) -> Vec<Action> {
    let root = app.paths.staging.join(&entry.name);
    let files = settings_files(details);
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
