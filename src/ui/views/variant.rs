//! Option picker for multi-variant mods in the details panel: option
//! folders, or alternative JSON files shown by their titles.

use crate::mod_engine::model::{ModDetails, ModEntry};
use crate::mod_engine::variants;
use crate::ui::theme::{self, accent};
use crate::ui::{Action, ModAction};
use eframe::egui::{self, RichText};

/// A combo box listing the mod's option folders; empty for normal mods.
pub fn show(ui: &mut egui::Ui, entry: &ModEntry, details: &ModDetails) -> Vec<Action> {
    if details.variants.is_empty() {
        return Vec::new();
    }
    let current = variants::effective(&details.variants, entry.variant.as_deref())
        .cloned()
        .unwrap_or_default();
    let mut chosen = current.clone();
    let label = |v: &String| {
        details
            .variants
            .iter()
            .position(|x| x == v)
            .and_then(|i| details.variant_labels.get(i))
            .unwrap_or(v)
            .clone()
    };
    ui.horizontal(|ui| {
        theme::badge(ui, "OPTIONS", accent());
        egui::ComboBox::from_id_salt(("variant", &entry.name))
            .selected_text(label(&chosen))
            .show_ui(ui, |ui| {
                details.variants.iter().for_each(|v| {
                    ui.selectable_value(&mut chosen, v.clone(), label(v))
                        .on_hover_text(v);
                });
            });
    });
    let files = details.variants.iter().all(|v| v.ends_with(".json"));
    let note = match files {
        true => {
            "This mod ships alternative JSON files that change the same values; only the chosen one is applied."
        }
        false => "Only the chosen option is deployed; files outside the option folders always are.",
    };
    ui.label(RichText::new(note).small());
    match chosen == current {
        true => Vec::new(),
        false => vec![Action::Mod(ModAction::SetVariant(
            entry.name.clone(),
            chosen,
        ))],
    }
}
