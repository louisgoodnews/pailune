//! Viewing and editing a mod's JSON patches in its details panel: a table
//! of the changes (new bytes editable as hex or as a number) and the raw
//! JSON, both checked live the way Apply reads them. Unsaved text lives in
//! egui's memory per file, like the plugin settings editor.

mod changes;

use crate::mod_engine::model::{ModDetails, ModEntry};
use crate::mod_engine::plan::Placement;
use crate::patch::{JsonMod, document};
use crate::ui::theme::{danger, muted, success};
use crate::ui::{Action, ModAction, PailuneApp};
use eframe::egui::{self, RichText};
use std::path::{Path, PathBuf};

/// An edit in progress: the file text it started from, the text now, and
/// whether the raw JSON is shown instead of the table.
#[derive(Clone, Default)]
struct Draft {
    base: String,
    text: String,
    raw: bool,
}

/// One line about the document: what it changes, or why Apply would fail.
fn summary(ui: &mut egui::Ui, checked: &Result<JsonMod, String>) {
    match checked {
        Ok(json) => {
            let bytes: usize = json.patches.iter().map(|p| p.changes.len()).sum();
            let fields: usize = json.patches.iter().map(|p| p.fields.len()).sum();
            let title = match json.title.is_empty() {
                true => String::new(),
                false => format!("{} — ", json.title),
            };
            ui.label(
                RichText::new(format!(
                    "✔ {title}{} change(s) in {} game file(s)",
                    bytes + fields,
                    json.patches.len()
                ))
                .color(success()),
            );
        }
        Err(err) => drop(ui.label(RichText::new(format!("✖ {err}")).color(danger()))),
    }
}

/// Save, Revert and Format; Save needs a document Apply can read.
fn buttons(
    ui: &mut egui::Ui,
    draft: &mut Draft,
    saved: &str,
    valid: bool,
    path: &Path,
) -> Option<Action> {
    let changed = draft.text != saved;
    let mut action = None;
    ui.horizontal(|ui| {
        let save = ui
            .add_enabled(changed && valid, egui::Button::new("💾 Save"))
            .on_disabled_hover_text("Nothing changed, or the file has an error");
        if save.clicked() {
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
            draft.text = saved.to_string();
            draft.base = saved.to_string();
        }
        let parsed = serde_json::from_str::<serde_json::Value>(&draft.text);
        if ui
            .add_enabled(parsed.is_ok(), egui::Button::new("Format"))
            .on_hover_text("Indent the JSON evenly")
            .clicked()
        {
            draft.text = parsed.map(|v| document::pretty(&v)).unwrap_or_default();
        }
        ui.separator();
        if ui
            .button("🔍 Browse data")
            .on_hover_text("Open the Game Data Browser to inspect vanilla table records")
            .clicked()
        {
            action = Some(Action::SwitchTab(crate::ui::Tab::DataBrowser));
        }
        ui.selectable_value(&mut draft.raw, false, "Changes");
        ui.selectable_value(&mut draft.raw, true, "Raw JSON");
    });
    action
}

/// One JSON file: summary, table or raw text, and the buttons. The file is
/// read only while its section is open; a mod's only file starts open.
fn editor(ui: &mut egui::Ui, path: &Path, rel: &Path, only: bool) -> Option<Action> {
    let id = egui::Id::unique(("pailune_json", path));
    let mut action = None;
    egui::CollapsingHeader::new(rel.display().to_string())
        .id_salt(id)
        .default_open(only)
        .show(ui, |ui| {
            let saved = std::fs::read_to_string(path).unwrap_or_default();
            let mut draft: Draft = ui.data(|d| d.get_temp(id)).unwrap_or_default();
            // Untouched drafts follow the file on disk.
            if draft.text == draft.base && draft.base != saved {
                draft.base = saved.clone();
                draft.text = saved.clone();
            }
            if draft.base != saved {
                ui.label(
                    RichText::new("The file changed since you started editing — Revert loads it.")
                        .color(danger()),
                );
            }
            let checked = document::check(&draft.text);
            summary(ui, &checked);
            action = buttons(ui, &mut draft, &saved, checked.is_ok(), path);
            match (&checked, draft.raw) {
                (Ok(json), false) => changes::show(ui, id, &mut draft.text, json),
                (Err(_), false) => drop(ui.label(
                    RichText::new("Fix the error in “Raw JSON” to see the changes.").color(muted()),
                )),
                (_, true) => drop(
                    ui.add(
                        egui::TextEdit::multiline(&mut draft.text)
                            .code_editor()
                            .desired_rows(14)
                            .desired_width(f32::INFINITY),
                    ),
                ),
            }
            ui.data_mut(|d| d.insert_temp(id, draft));
        });
    action
}

/// The JSON patch files among the mod's (already listed) files.
fn json_files(details: &ModDetails) -> Vec<PathBuf> {
    details
        .files
        .iter()
        .filter(|f| f.placement == Placement::JsonPatch)
        .map(|f| f.rel.clone())
        .collect()
}

/// The JSON patches section of a mod; empty when it ships none.
pub fn show(
    ui: &mut egui::Ui,
    entry: &ModEntry,
    details: &ModDetails,
    app: &PailuneApp,
) -> Vec<Action> {
    let files = json_files(details);
    if files.is_empty() {
        return Vec::new();
    }
    let root = app.paths.staging.join(&entry.name);
    ui.separator();
    ui.label(RichText::new("JSON patches").strong());
    ui.label(
        RichText::new("Edits take effect with the next Apply. Original bytes are checked first.")
            .color(muted()),
    );
    files
        .iter()
        .filter_map(|rel| editor(ui, &root.join(rel), rel, files.len() == 1))
        .collect()
}
