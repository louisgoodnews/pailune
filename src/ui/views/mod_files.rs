//! File list and readme buttons of the mod details panel.

use super::clicked;
use crate::mod_engine::model::ModDetails;
use crate::mod_engine::plan::Placement;
use crate::ui::theme::{MUTED, WARNING};
use crate::ui::{Action, ModAction};
use eframe::egui::{self, RichText};

/// Renders readme buttons and the collapsible file list.
pub fn show(ui: &mut egui::Ui, details: &ModDetails) -> Vec<Action> {
    let readmes = clicked(details.readmes.iter().map(|rel| {
        let open = ui.button(format!("📄 {}", rel.display())).clicked();
        (open, Action::Mod(ModAction::ShowReadme(rel.clone())))
    }));
    egui::CollapsingHeader::new(format!("All files ({})", details.files.len())).show(ui, |ui| {
        details.files.iter().for_each(|f| {
            let color = match f.placement {
                Placement::Mirror
                | Placement::Bin64
                | Placement::Pack
                | Placement::PrebuiltGroup => ui.visuals().text_color(),
                Placement::NeedsPaz(_) => WARNING,
                Placement::Skip => MUTED,
            };
            ui.label(
                RichText::new(format!("{}  ({} B)", f.rel.display(), f.bytes))
                    .small()
                    .monospace()
                    .color(color),
            )
            .on_hover_text(f.placement.note());
        });
    });
    readmes
}
