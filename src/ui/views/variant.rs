//! Option picker for multi-variant mods in the details panel.

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
    ui.horizontal(|ui| {
        theme::badge(ui, "OPTIONS", accent());
        egui::ComboBox::from_id_salt(("variant", &entry.name))
            .selected_text(&chosen)
            .show_ui(ui, |ui| {
                details
                    .variants
                    .iter()
                    .for_each(|v| drop(ui.selectable_value(&mut chosen, v.clone(), v)));
            });
    });
    ui.label(
        RichText::new(
            "Only the chosen option is deployed; files outside the option folders always are.",
        )
        .small(),
    );
    match chosen == current {
        true => Vec::new(),
        false => vec![Action::Mod(ModAction::SetVariant(
            entry.name.clone(),
            chosen,
        ))],
    }
}
